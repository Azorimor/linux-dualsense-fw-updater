// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

export const GITHUB_RELEASE_API_URL =
  'https://api.github.com/repos/Azorimor/linux-dualsense-fw-updater/releases/latest';

export type ReleaseAssetKind = 'appimage' | 'tarball' | 'checksums';

export interface ReleaseAsset {
  name: string;
  size: number;
  formattedSize: string;
  downloadUrl: string;
  kind: ReleaseAssetKind;
}

export interface GitHubRelease {
  tagName: string;
  name: string;
  publishedAt: string;
  htmlUrl: string;
  assets: ReleaseAsset[];
}

const ASSET_ORDER: Record<ReleaseAssetKind, number> = {
  appimage: 0,
  tarball: 1,
  checksums: 2,
};

export function classifyReleaseAsset(name: string): ReleaseAssetKind | null {
  const lower = name.toLowerCase();
  if (lower.endsWith('.appimage')) {
    return 'appimage';
  }
  if (lower.endsWith('.tar.gz')) {
    return 'tarball';
  }
  if (name === 'SHA256SUMS.txt' || lower === 'sha256sums.txt') {
    return 'checksums';
  }
  return null;
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return '0 B';
  }
  const units = ['B', 'KiB', 'MiB', 'GiB'];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  const formatted = unitIndex === 0 ? `${Math.round(value)}` : value.toFixed(1);
  return `${formatted} ${units[unitIndex]}`;
}

export function formatPublishedDate(iso: string): string {
  if (!iso) {
    return 'Unknown date';
  }
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) {
    return iso;
  }
  return new Intl.DateTimeFormat('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    timeZone: 'UTC',
  }).format(parsed);
}

export function buildGitHubHeaders(token?: string): Record<string, string> {
  const headers: Record<string, string> = {
    Accept: 'application/vnd.github+json',
    'User-Agent': 'linux-dualsense-fw-updater-website',
    'X-GitHub-Api-Version': '2022-11-28',
  };
  const trimmed = token?.trim();
  if (trimmed) {
    headers.Authorization = `Bearer ${trimmed}`;
  }
  return headers;
}

function isHttpsUrl(value: unknown): value is string {
  return typeof value === 'string' && value.startsWith('https://');
}

export function parseGitHubRelease(payload: unknown): GitHubRelease | null {
  if (typeof payload !== 'object' || payload === null) {
    return null;
  }

  const record = payload as Record<string, unknown>;
  if (record.draft === true) {
    return null;
  }

  const tagName =
    typeof record.tag_name === 'string' ? record.tag_name.trim() : '';
  const htmlUrl = isHttpsUrl(record.html_url) ? record.html_url : '';
  if (!tagName || !htmlUrl) {
    return null;
  }

  const rawName = typeof record.name === 'string' ? record.name.trim() : '';
  const name = rawName || tagName;
  const publishedAt =
    typeof record.published_at === 'string' && record.published_at.trim()
      ? record.published_at.trim()
      : typeof record.created_at === 'string'
        ? record.created_at.trim()
        : '';

  const rawAssets = Array.isArray(record.assets) ? record.assets : [];
  const assets: ReleaseAsset[] = [];

  for (const item of rawAssets) {
    if (typeof item !== 'object' || item === null) {
      continue;
    }
    const asset = item as Record<string, unknown>;
    const assetName =
      typeof asset.name === 'string' ? asset.name.trim() : '';
    const downloadUrl = isHttpsUrl(asset.browser_download_url)
      ? asset.browser_download_url
      : '';
    const size =
      typeof asset.size === 'number' && Number.isFinite(asset.size)
        ? Math.max(0, asset.size)
        : 0;
    const kind = assetName ? classifyReleaseAsset(assetName) : null;

    if (!assetName || !downloadUrl || !kind) {
      continue;
    }

    assets.push({
      name: assetName,
      size,
      formattedSize: formatBytes(size),
      downloadUrl,
      kind,
    });
  }

  assets.sort((a, b) => ASSET_ORDER[a.kind] - ASSET_ORDER[b.kind]);

  return {
    tagName,
    name,
    publishedAt,
    htmlUrl,
    assets,
  };
}

export interface FetchLatestReleaseOptions {
  token?: string;
  fetchImpl?: typeof fetch;
  apiUrl?: string;
  timeoutMs?: number;
}

export async function fetchLatestRelease(
  options: FetchLatestReleaseOptions = {},
): Promise<GitHubRelease | null> {
  const fetchFn = options.fetchImpl ?? fetch;
  const apiUrl = options.apiUrl ?? GITHUB_RELEASE_API_URL;
  const token = options.token ?? process.env.GITHUB_TOKEN;
  const timeoutMs = options.timeoutMs ?? 8000;

  try {
    const response = await fetchFn(apiUrl, {
      headers: buildGitHubHeaders(token),
      signal: AbortSignal.timeout(timeoutMs),
    });
    if (!response.ok) {
      return null;
    }
    const data: unknown = await response.json();
    return parseGitHubRelease(data);
  } catch {
    return null;
  }
}
