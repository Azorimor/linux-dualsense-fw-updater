use crate::cancel::CancellationToken;
use crate::error::{CoreError, Result};
use crate::image::FirmwareImage;

/// Official Sony DualSense firmware catalog URL.
pub const SONY_INFO_JSON_URL: &str = "https://fwupdater.dl.playstation.net/fwupdater/info.json";

/// Official Sony DualSense firmware binary base URL.
pub const SONY_FW_BASE_URL: &str = "https://fwupdater.dl.playstation.net/fwupdater";

/// Metadata describing a known DualSense hardware target ID.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FirmwareTargetInfo {
    pub target_id: &'static str,
    pub board_revisions: &'static str,
    pub model_name: &'static str,
    pub tested: bool,
}

/// Known DualSense hardware target mappings documented for Sony's firmware catalog.
pub const KNOWN_TARGETS: &[FirmwareTargetInfo] = &[
    FirmwareTargetInfo {
        target_id: "0004",
        board_revisions: "BDM-020 / BDM-030 / BDM-040",
        model_name: "DualSense",
        tested: true,
    },
    FirmwareTargetInfo {
        target_id: "000B",
        board_revisions: "BDM-050",
        model_name: "DualSense",
        tested: true,
    },
    FirmwareTargetInfo {
        target_id: "000E",
        board_revisions: "BDM-060",
        model_name: "DualSense",
        tested: true,
    },
    FirmwareTargetInfo {
        target_id: "0044",
        board_revisions: "DualSense Edge",
        model_name: "DualSense Edge",
        tested: false,
    },
];

/// Looks up metadata for a known target ID (e.g. `"0004"`, `"000B"`).
pub fn find_known_target(target_id: &str) -> Option<FirmwareTargetInfo> {
    KNOWN_TARGETS
        .iter()
        .find(|t| t.target_id.eq_ignore_ascii_case(target_id))
        .copied()
}

/// Byte progress snapshot emitted while downloading a firmware `.bin` image.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DownloadProgress {
    pub downloaded_bytes: usize,
    pub total_bytes: Option<usize>,
}

/// Synchronous HTTP client abstraction for fetching Sony's catalog and firmware images.
pub trait HttpClient {
    /// Fetches UTF-8 text from `url`, aborting early if `cancel` is triggered.
    fn get_text(&self, url: &str, cancel: &CancellationToken) -> Result<String>;

    /// Downloads raw bytes from `url`, emitting progress updates and respecting `cancel`.
    fn download_with_progress(
        &self,
        url: &str,
        cancel: &CancellationToken,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<Vec<u8>>;
}

/// Single firmware release entry parsed from Sony's `info.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareRelease {
    pub target_id: String,
    pub version_raw: String,
    pub version: u16,
    pub download_url: String,
    pub target_info: Option<FirmwareTargetInfo>,
}

/// Parsed Sony firmware catalog (`info.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareCatalog {
    pub application_latest_version: Option<String>,
    pub releases: Vec<FirmwareRelease>,
}

impl FirmwareCatalog {
    /// Parses Sony's `info.json` payload into a [`FirmwareCatalog`].
    pub fn from_json(json: &str) -> Result<Self> {
        let pairs = parse_flat_json_object(json)?;
        let mut application_latest_version = None;
        let mut releases = Vec::new();

        for (key, value) in pairs {
            if key == "ApplicationLatestVersion" {
                application_latest_version = Some(value);
                continue;
            }
            if let Some(target_id) = extract_target_id(&key) {
                let (version, version_raw) = parse_hex_version(&value)?;
                let download_url = build_firmware_url(target_id, &version_raw)?;
                releases.push(FirmwareRelease {
                    target_id: target_id.to_string(),
                    version_raw,
                    version,
                    download_url,
                    target_info: find_known_target(target_id),
                });
            }
        }

        if releases.is_empty() {
            return Err(CoreError::CatalogParse(
                "No FwUpdate<TARGET>LatestVersion entries found".to_string(),
            ));
        }

        releases.sort_by(|a, b| a.target_id.cmp(&b.target_id));
        Ok(Self {
            application_latest_version,
            releases,
        })
    }

    /// Finds the firmware release for `target_id` (case-insensitive).
    pub fn release_for_target(&self, target_id: &str) -> Result<&FirmwareRelease> {
        self.releases
            .iter()
            .find(|r| r.target_id.eq_ignore_ascii_case(target_id))
            .ok_or_else(|| CoreError::CatalogTargetNotFound(target_id.to_string()))
    }
}

/// Builds the Sony firmware `.bin` download URL while preserving the required `0x` prefix.
pub fn build_firmware_url(target_id: &str, version_raw: &str) -> Result<String> {
    if target_id.is_empty() || !target_id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(CoreError::CatalogParse(format!(
            "Invalid firmware target ID: {target_id:?}"
        )));
    }
    let (_, normalized) = parse_hex_version(version_raw)?;
    Ok(format!(
        "{SONY_FW_BASE_URL}/fwupdate{target_id}/{normalized}/FWUPDATE{target_id}.bin"
    ))
}

/// High-level client for fetching Sony's catalog and downloading validated [`FirmwareImage`]s.
#[derive(Debug)]
pub struct FirmwareCatalogClient<C> {
    http: C,
}

impl<C: HttpClient> FirmwareCatalogClient<C> {
    /// Wraps an [`HttpClient`] implementation.
    pub fn new(http: C) -> Self {
        Self { http }
    }

    /// Fetches and parses Sony's `info.json` from [`SONY_INFO_JSON_URL`].
    pub fn fetch_catalog(&self, cancel: &CancellationToken) -> Result<FirmwareCatalog> {
        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        let body = self.http.get_text(SONY_INFO_JSON_URL, cancel)?;
        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        FirmwareCatalog::from_json(&body)
    }

    /// Downloads raw firmware bytes for `release`, forwarding byte progress and checking `cancel`.
    pub fn download_firmware_bytes(
        &self,
        release: &FirmwareRelease,
        cancel: &CancellationToken,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<Vec<u8>> {
        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        let bytes = self
            .http
            .download_with_progress(&release.download_url, cancel, on_progress)?;
        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        Ok(bytes)
    }

    /// Downloads and validates a [`FirmwareImage`] for `release`.
    pub fn download_firmware(
        &self,
        release: &FirmwareRelease,
        cancel: &CancellationToken,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<FirmwareImage> {
        let bytes = self.download_firmware_bytes(release, cancel, on_progress)?;
        FirmwareImage::from_bytes(bytes)
    }
}

fn extract_target_id(key: &str) -> Option<&str> {
    let rest = key.strip_prefix("FwUpdate")?;
    let target = rest.strip_suffix("LatestVersion")?;
    if target.is_empty() {
        None
    } else {
        Some(target)
    }
}

fn parse_hex_version(raw: &str) -> Result<(u16, String)> {
    let trimmed = raw.trim();
    let hex_digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    if hex_digits.is_empty() || hex_digits.len() > 4 {
        return Err(CoreError::CatalogParse(format!(
            "Invalid hex firmware version: {raw:?}"
        )));
    }
    let version = u16::from_str_radix(hex_digits, 16)
        .map_err(|_| CoreError::CatalogParse(format!("Invalid hex firmware version: {raw:?}")))?;
    let canonical = if trimmed.starts_with("0x") {
        trimmed.to_string()
    } else {
        format!("0x{hex_digits}")
    };
    Ok((version, canonical))
}

fn parse_flat_json_object(input: &str) -> Result<Vec<(String, String)>> {
    let mut chars = input.char_indices().peekable();
    skip_ws(&mut chars);
    expect_char(&mut chars, '{')?;

    let mut pairs = Vec::new();
    skip_ws(&mut chars);
    if matches!(chars.peek(), Some((_, '}'))) {
        chars.next();
        skip_ws(&mut chars);
        if chars.peek().is_some() {
            return Err(CoreError::CatalogParse(
                "Trailing characters after JSON object".to_string(),
            ));
        }
        return Ok(pairs);
    }

    loop {
        skip_ws(&mut chars);
        let key = parse_json_string(&mut chars)?;
        skip_ws(&mut chars);
        expect_char(&mut chars, ':')?;
        skip_ws(&mut chars);
        let value = parse_json_string(&mut chars)?;
        pairs.push((key, value));

        skip_ws(&mut chars);
        match chars.next() {
            Some((_, ',')) => continue,
            Some((_, '}')) => break,
            other => {
                return Err(CoreError::CatalogParse(format!(
                    "Expected ',' or '}}' in JSON object, got {other:?}"
                )));
            }
        }
    }

    skip_ws(&mut chars);
    if chars.peek().is_some() {
        return Err(CoreError::CatalogParse(
            "Trailing characters after JSON object".to_string(),
        ));
    }
    Ok(pairs)
}

fn skip_ws(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) {
    while matches!(chars.peek(), Some((_, c)) if c.is_whitespace()) {
        chars.next();
    }
}

fn expect_char(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    expected: char,
) -> Result<()> {
    match chars.next() {
        Some((_, c)) if c == expected => Ok(()),
        other => Err(CoreError::CatalogParse(format!(
            "Expected '{expected}', got {other:?}"
        ))),
    }
}

fn parse_json_string(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Result<String> {
    expect_char(chars, '"')?;
    let mut out = String::new();
    while let Some((_, c)) = chars.next() {
        match c {
            '"' => return Ok(out),
            '\\' => match chars.next() {
                Some((_, '"')) => out.push('"'),
                Some((_, '\\')) => out.push('\\'),
                Some((_, '/')) => out.push('/'),
                Some((_, 'n')) => out.push('\n'),
                Some((_, 'r')) => out.push('\r'),
                Some((_, 't')) => out.push('\t'),
                Some((_, other)) => {
                    return Err(CoreError::CatalogParse(format!(
                        "Unsupported JSON escape '\\{other}'"
                    )));
                }
                None => {
                    return Err(CoreError::CatalogParse(
                        "Unterminated JSON escape".to_string(),
                    ));
                }
            },
            c if c.is_control() => {
                return Err(CoreError::CatalogParse(
                    "Unescaped control character in JSON string".to_string(),
                ));
            }
            c => out.push(c),
        }
    }
    Err(CoreError::CatalogParse(
        "Unterminated JSON string".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::FIRMWARE_VERSION_OFFSET;

    const SAMPLE_INFO_JSON: &str = r#"{"ApplicationLatestVersion":"2.2.1.2","FwUpdate0004LatestVersion":"0x0630","FwUpdate000BLatestVersion":"0x0630","FwUpdate000ELatestVersion":"0x0641","FwUpdate0044LatestVersion":"0x0217"}"#;

    #[test]
    fn parses_sony_info_json_and_builds_urls_with_0x_prefix() {
        let catalog = FirmwareCatalog::from_json(SAMPLE_INFO_JSON).expect("valid catalog");
        assert_eq!(
            catalog.application_latest_version.as_deref(),
            Some("2.2.1.2")
        );
        assert_eq!(catalog.releases.len(), 4);

        let bdm020 = catalog.release_for_target("0004").expect("0004 release");
        assert_eq!(bdm020.version, 0x0630);
        assert_eq!(bdm020.version_raw, "0x0630");
        assert_eq!(
            bdm020.download_url,
            "https://fwupdater.dl.playstation.net/fwupdater/fwupdate0004/0x0630/FWUPDATE0004.bin"
        );
        assert!(bdm020.target_info.expect("target info").tested);

        let bdm050 = catalog.release_for_target("000b").expect("000B release");
        assert_eq!(bdm050.target_id, "000B");
        assert_eq!(bdm050.version, 0x0630);

        let bdm060 = catalog.release_for_target("000E").expect("000E release");
        assert_eq!(bdm060.version, 0x0641);

        let edge = catalog.release_for_target("0044").expect("0044 release");
        assert_eq!(edge.version, 0x0217);
        assert!(!edge.target_info.expect("edge info").tested);

        assert_eq!(
            catalog.release_for_target("9999"),
            Err(CoreError::CatalogTargetNotFound("9999".to_string()))
        );
    }

    #[test]
    fn rejects_invalid_json_and_invalid_versions() {
        assert!(matches!(
            FirmwareCatalog::from_json("not json"),
            Err(CoreError::CatalogParse(_))
        ));
        assert!(matches!(
            FirmwareCatalog::from_json("{}"),
            Err(CoreError::CatalogParse(_))
        ));
        assert!(matches!(
            FirmwareCatalog::from_json(r#"{"FwUpdate0004LatestVersion":"not_hex"}"#),
            Err(CoreError::CatalogParse(_))
        ));
        assert!(matches!(
            build_firmware_url("0004/../bad", "0x0630"),
            Err(CoreError::CatalogParse(_))
        ));
        assert_eq!(
            build_firmware_url("000B", "0630").expect("normalizes 0x prefix"),
            "https://fwupdater.dl.playstation.net/fwupdater/fwupdate000B/0x0630/FWUPDATE000B.bin"
        );
    }

    struct MockHttpClient {
        json_response: Result<String>,
        download_chunks: Vec<Vec<u8>>,
        cancel_after_first_chunk: bool,
    }

    impl HttpClient for MockHttpClient {
        fn get_text(&self, url: &str, _cancel: &CancellationToken) -> Result<String> {
            assert_eq!(url, SONY_INFO_JSON_URL);
            self.json_response.clone()
        }

        fn download_with_progress(
            &self,
            _url: &str,
            cancel: &CancellationToken,
            on_progress: &mut dyn FnMut(DownloadProgress),
        ) -> Result<Vec<u8>> {
            let total: usize = self.download_chunks.iter().map(Vec::len).sum();
            let mut out = Vec::with_capacity(total);
            for (idx, chunk) in self.download_chunks.iter().enumerate() {
                if cancel.is_cancelled() {
                    return Err(CoreError::Cancelled);
                }
                out.extend_from_slice(chunk);
                on_progress(DownloadProgress {
                    downloaded_bytes: out.len(),
                    total_bytes: Some(total),
                });
                if idx == 0 && self.cancel_after_first_chunk {
                    cancel.cancel();
                }
            }
            Ok(out)
        }
    }

    #[test]
    fn catalog_client_fetches_downloads_and_validates_firmware_image() {
        let mut fw_bytes = vec![0xAAu8; 512];
        fw_bytes[FIRMWARE_VERSION_OFFSET..FIRMWARE_VERSION_OFFSET + 2]
            .copy_from_slice(&0x0641u16.to_le_bytes());

        let client = FirmwareCatalogClient::new(MockHttpClient {
            json_response: Ok(SAMPLE_INFO_JSON.to_string()),
            download_chunks: vec![fw_bytes[..200].to_vec(), fw_bytes[200..].to_vec()],
            cancel_after_first_chunk: false,
        });
        let cancel = CancellationToken::new();
        let catalog = client.fetch_catalog(&cancel).expect("catalog");
        let release = catalog.release_for_target("000E").expect("000E");

        let mut progress = Vec::new();
        let image = client
            .download_firmware(release, &cancel, &mut |p| progress.push(p))
            .expect("valid image");
        assert_eq!(image.version(), 0x0641);
        assert_eq!(image.len(), 512);
        assert_eq!(
            progress,
            vec![
                DownloadProgress {
                    downloaded_bytes: 200,
                    total_bytes: Some(512)
                },
                DownloadProgress {
                    downloaded_bytes: 512,
                    total_bytes: Some(512)
                },
            ]
        );
    }

    #[test]
    fn catalog_client_respects_cancellation_before_and_during_download() {
        let client = FirmwareCatalogClient::new(MockHttpClient {
            json_response: Ok(SAMPLE_INFO_JSON.to_string()),
            download_chunks: vec![vec![0u8; 128], vec![0u8; 128]],
            cancel_after_first_chunk: true,
        });
        let cancel = CancellationToken::new();
        let catalog = client.fetch_catalog(&cancel).expect("catalog");
        let release = catalog.release_for_target("0004").expect("0004");

        let err = client
            .download_firmware(release, &cancel, &mut |_| {})
            .unwrap_err();
        assert_eq!(err, CoreError::Cancelled);
        assert_eq!(client.fetch_catalog(&cancel), Err(CoreError::Cancelled));
    }
}
