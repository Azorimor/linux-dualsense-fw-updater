use crate::cancel::CancellationToken;
use crate::error::{CoreError, Result};
use crate::image::FirmwareImage;
use crate::protocol::{BoardModel, DUALSENSE_EDGE_PID, FirmwareInfo};

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

/// Maps a `fw_series` value from Feature Report `0x20` to Sony's firmware catalog target ID.
pub fn target_id_for_fw_series(fw_series: u16) -> Option<&'static str> {
    match fw_series {
        0x0004 => Some("0004"),
        0x000B => Some("000B"),
        0x000E => Some("000E"),
        0x0044 => Some("0044"),
        _ => None,
    }
}

/// Hardware board and Sony firmware target resolution for a connected controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerHardwareInfo {
    pub board_model: BoardModel,
    pub board_name: String,
    pub fw_type: u16,
    pub fw_series: u16,
    pub fw_series_hex: String,
    pub hw_info: u32,
    pub fw_version: u32,
    pub supported_target_id: Option<&'static str>,
    pub is_officially_supported: bool,
    pub unsupported_reason: Option<String>,
}

/// Resolves the hardware board revision and official Sony firmware target from `0x20` info and USB PID.
pub fn resolve_controller_target(info: &FirmwareInfo, product_id: u16) -> ControllerHardwareInfo {
    let fw_type = info.fw_type();
    let fw_series = info.fw_series();
    let fw_series_hex = info.formatted_fw_series();
    let hw_info = info.hw_info();
    let fw_version = info.fw_version();
    let board_model = info.board_model();
    let series_target = target_id_for_fw_series(fw_series);

    if product_id == DUALSENSE_EDGE_PID {
        let board_name = "DualSense Edge".to_string();
        let (supported_target_id, is_officially_supported, unsupported_reason) = if fw_series == 0
            || fw_series == 0x0044
        {
            (Some("0044"), true, None)
        } else {
            (
                None,
                false,
                Some(format!(
                    "This DualSense Edge reported firmware series {fw_series_hex}, which does not match Sony's DualSense Edge update target (0044)."
                )),
            )
        };

        return ControllerHardwareInfo {
            board_model,
            board_name,
            fw_type,
            fw_series,
            fw_series_hex,
            hw_info,
            fw_version,
            supported_target_id,
            is_officially_supported,
            unsupported_reason,
        };
    }

    let board_name = board_model.as_str().to_string();
    let (supported_target_id, is_officially_supported, unsupported_reason) =
        match board_model.sony_target_id() {
            Some(board_target) => {
                if fw_series == 0 || series_target == Some(board_target) {
                    (Some(board_target), true, None)
                } else if series_target.is_none() {
                    (
                        None,
                        false,
                        Some(format!(
                            "This {board_name} controller reported firmware series {fw_series_hex}, which is not in Sony's official update catalog."
                        )),
                    )
                } else {
                    (
                        None,
                        false,
                        Some(format!(
                            "Controller board {board_name} does not match reported firmware series {fw_series_hex}."
                        )),
                    )
                }
            }
            None => match board_model {
                BoardModel::Bdm010 => (
                    None,
                    false,
                    Some(
                        "DualSense BDM-010 controllers are not supported by Sony's official PC firmware catalog."
                            .to_string(),
                    ),
                ),
                _ => {
                    if hw_info == 0 && fw_series == 0 {
                        (
                            None,
                            false,
                            Some(
                                "The controller did not report a hardware board revision or firmware series."
                                    .to_string(),
                            ),
                        )
                    } else if let Some(target) = series_target {
                        if target == "0044" {
                            (
                                None,
                                false,
                                Some(format!(
                                    "Controller reported DualSense Edge firmware series {fw_series_hex}, but is not a DualSense Edge USB device."
                                )),
                            )
                        } else {
                            (Some(target), true, None)
                        }
                    } else {
                        (
                            None,
                            false,
                            Some(format!(
                                "Unrecognized controller hardware (0x{hw_info:08X}, firmware series {fw_series_hex}) is not in Sony's official update catalog."
                            )),
                        )
                    }
                }
            },
        };

    ControllerHardwareInfo {
        board_model,
        board_name,
        fw_type,
        fw_series,
        fw_series_hex,
        hw_info,
        fw_version,
        supported_target_id,
        is_officially_supported,
        unsupported_reason,
    }
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

fn parse_json_string(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) -> Result<String> {
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

    fn make_hw_firmware_info(
        fw_type: u16,
        fw_series: u16,
        hw_info: u32,
        fw_version: u32,
        upd_version: u16,
    ) -> FirmwareInfo {
        let mut raw = vec![0u8; 64];
        raw[0] = crate::protocol::REPORT_ID_FIRMWARE_INFO;
        raw[1..12].copy_from_slice(b"Oct  1 2024");
        raw[12..20].copy_from_slice(b"12:00:00");
        raw[20..22].copy_from_slice(&fw_type.to_le_bytes());
        raw[22..24].copy_from_slice(&fw_series.to_le_bytes());
        raw[24..28].copy_from_slice(&hw_info.to_le_bytes());
        raw[28..32].copy_from_slice(&fw_version.to_le_bytes());
        raw[44..46].copy_from_slice(&upd_version.to_le_bytes());
        FirmwareInfo::from_report(raw).expect("valid firmware info")
    }

    #[test]
    fn resolve_controller_target_maps_all_supported_boards_and_edge() {
        use crate::protocol::DEFAULT_PID;

        let bdm050 = make_hw_firmware_info(0x0003, 0x000B, 0x0000_0811, 0x0001_0630, 0x0630);
        let hw050 = resolve_controller_target(&bdm050, DEFAULT_PID);
        assert_eq!(hw050.board_model, BoardModel::Bdm050);
        assert_eq!(hw050.board_name, "BDM-050");
        assert_eq!(hw050.fw_series, 0x000B);
        assert_eq!(hw050.fw_series_hex, "0x000B");
        assert_eq!(hw050.hw_info, 0x0000_0811);
        assert_eq!(hw050.supported_target_id, Some("000B"));
        assert!(hw050.is_officially_supported);
        assert_eq!(hw050.unsupported_reason, None);

        for (hw_info, expected_name) in [
            (0x0000_0400, "BDM-020"),
            (0x0000_0500, "BDM-030"),
            (0x0000_0600, "BDM-040"),
        ] {
            let info = make_hw_firmware_info(0x0002, 0x0004, hw_info, 0x0001_0630, 0x0630);
            let resolved = resolve_controller_target(&info, DEFAULT_PID);
            assert_eq!(resolved.board_name, expected_name);
            assert_eq!(resolved.supported_target_id, Some("0004"));
            assert!(resolved.is_officially_supported);
        }

        for (hw_info, expected_name) in [
            (0x0000_0900, "BDM-060R"),
            (0x0000_1100, "BDM-060M"),
            (0x0000_1300, "BDM-060X"),
        ] {
            let info = make_hw_firmware_info(0x0002, 0x000E, hw_info, 0x0001_0641, 0x0641);
            let resolved = resolve_controller_target(&info, DEFAULT_PID);
            assert_eq!(resolved.board_name, expected_name);
            assert_eq!(resolved.supported_target_id, Some("000E"));
            assert!(resolved.is_officially_supported);
        }

        // Known board with zeroed fw_series falls back to board mapping
        let bdm050_zero_series = make_hw_firmware_info(0, 0, 0x0000_0700, 0, 0x0500);
        let resolved_zero_series = resolve_controller_target(&bdm050_zero_series, DEFAULT_PID);
        assert_eq!(resolved_zero_series.supported_target_id, Some("000B"));
        assert!(resolved_zero_series.is_officially_supported);

        // DualSense Edge with matching 0x0044 or 0x0000 fw_series
        let edge = make_hw_firmware_info(0x0002, 0x0044, 0x0000_0400, 0x0001_0217, 0x0217);
        let hw_edge = resolve_controller_target(&edge, DUALSENSE_EDGE_PID);
        assert_eq!(hw_edge.board_name, "DualSense Edge");
        assert_eq!(hw_edge.supported_target_id, Some("0044"));
        assert!(hw_edge.is_officially_supported);

        let edge_zero = make_hw_firmware_info(0, 0, 0, 0, 0x0217);
        let hw_edge_zero = resolve_controller_target(&edge_zero, DUALSENSE_EDGE_PID);
        assert_eq!(hw_edge_zero.board_name, "DualSense Edge");
        assert_eq!(hw_edge_zero.supported_target_id, Some("0044"));
        assert!(hw_edge_zero.is_officially_supported);
    }

    #[test]
    fn resolve_controller_target_handles_unsupported_unknown_and_mismatched_boards() {
        use crate::protocol::DEFAULT_PID;

        // BDM-010 is unsupported by Sony PC catalog
        let bdm010 = make_hw_firmware_info(0x0001, 0x0001, 0x0000_0300, 0x0001_0200, 0x0200);
        let hw010 = resolve_controller_target(&bdm010, DEFAULT_PID);
        assert_eq!(hw010.board_model, BoardModel::Bdm010);
        assert_eq!(hw010.board_name, "BDM-010");
        assert_eq!(hw010.supported_target_id, None);
        assert!(!hw010.is_officially_supported);
        assert!(
            hw010
                .unsupported_reason
                .as_deref()
                .unwrap()
                .contains("BDM-010")
        );

        // Zeroed hw_info + fw_series on standard PID -> Unknown / unsupported
        let zeroed = make_hw_firmware_info(0, 0, 0, 0, 0x0500);
        let hw_zeroed = resolve_controller_target(&zeroed, DEFAULT_PID);
        assert_eq!(hw_zeroed.board_model, BoardModel::Unknown);
        assert_eq!(hw_zeroed.board_name, "Unknown");
        assert_eq!(hw_zeroed.supported_target_id, None);
        assert!(!hw_zeroed.is_officially_supported);
        assert!(hw_zeroed.unsupported_reason.is_some());

        // Unknown board with unsupported non-zero fw_series
        let unknown_bad = make_hw_firmware_info(0, 0x0099, 0x0000_1F00, 0, 0x0500);
        let hw_unknown_bad = resolve_controller_target(&unknown_bad, DEFAULT_PID);
        assert_eq!(hw_unknown_bad.supported_target_id, None);
        assert!(!hw_unknown_bad.is_officially_supported);
        assert!(hw_unknown_bad.unsupported_reason.is_some());

        // Unknown board with known Sony fw_series resolves via fw_series
        let unknown_known_series = make_hw_firmware_info(0, 0x000E, 0x0000_1900, 0, 0x0641);
        let hw_unknown_known = resolve_controller_target(&unknown_known_series, DEFAULT_PID);
        assert_eq!(hw_unknown_known.board_name, "Unknown");
        assert_eq!(hw_unknown_known.supported_target_id, Some("000E"));
        assert!(hw_unknown_known.is_officially_supported);

        // Known board with non-zero unknown fw_series -> unsupported
        let bdm050_bad_series = make_hw_firmware_info(0, 0x0077, 0x0000_0811, 0, 0x0630);
        let hw_bad_series = resolve_controller_target(&bdm050_bad_series, DEFAULT_PID);
        assert_eq!(hw_bad_series.supported_target_id, None);
        assert!(!hw_bad_series.is_officially_supported);
        assert!(hw_bad_series.unsupported_reason.is_some());

        // Known board with mismatched known fw_series -> unsupported
        let bdm050_mismatch = make_hw_firmware_info(0, 0x0004, 0x0000_0811, 0, 0x0630);
        let hw_mismatch = resolve_controller_target(&bdm050_mismatch, DEFAULT_PID);
        assert_eq!(hw_mismatch.supported_target_id, None);
        assert!(!hw_mismatch.is_officially_supported);
        assert!(hw_mismatch.unsupported_reason.is_some());

        // DualSense Edge with mismatched fw_series -> unsupported
        let edge_mismatch = make_hw_firmware_info(0, 0x0004, 0x0000_0400, 0, 0x0217);
        let hw_edge_mismatch = resolve_controller_target(&edge_mismatch, DUALSENSE_EDGE_PID);
        assert_eq!(hw_edge_mismatch.supported_target_id, None);
        assert!(!hw_edge_mismatch.is_officially_supported);
        assert!(hw_edge_mismatch.unsupported_reason.is_some());

        // Non-Edge device reporting Edge fw_series 0x0044 -> unsupported
        let non_edge_0044 = make_hw_firmware_info(0, 0x0044, 0x0000_1F00, 0, 0x0217);
        let hw_non_edge_0044 = resolve_controller_target(&non_edge_0044, DEFAULT_PID);
        assert_eq!(hw_non_edge_0044.supported_target_id, None);
        assert!(!hw_non_edge_0044.is_officially_supported);
    }
}
