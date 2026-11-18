// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

export const SITE_BASE_PATH = '/linux-dualsense-fw-updater';

/**
 * Convert a Fumadocs page URL (e.g. `/docs` or `/docs/getting-started`) to the
 * canonical Markdown path used by `fumadocs-core/source` `llms()`.
 */
export function getMarkdownPath(pageUrl: string): string {
  const normalized = pageUrl.trim();
  if (normalized === '' || normalized === '/') {
    return '/index.md';
  }
  return `${normalized.replace(/\/+$/, '')}.md`;
}

/**
 * Map an `llms()` link target to the static-export URL under GitHub Pages `basePath`.
 * Markdown and text artifacts live under `<basePath>/llms.mdx/...`.
 */
export function toStaticLlmsUrl(
  itemUrl: string,
  basePath = SITE_BASE_PATH,
): string {
  const prefix = basePath.replace(/\/+$/, '');
  if (/^\/.*\.(md|txt)$/.test(itemUrl)) {
    return `${prefix}/llms.mdx${itemUrl}`;
  }
  if (itemUrl.startsWith('/')) {
    return `${prefix}${itemUrl}`;
  }
  return itemUrl;
}

/**
 * Build the static-export `.md` URL for a docs page (including `basePath`).
 */
export function getStaticPageMarkdownUrl(
  pageUrl: string,
  basePath = SITE_BASE_PATH,
): string {
  const prefix = basePath.replace(/\/+$/, '');
  return `${prefix}/llms.mdx${getMarkdownPath(pageUrl)}`;
}

/**
 * Build the GitHub source link for a docs page stored in `website/content/docs/`.
 */
export function getDocGitHubUrl(repoUrl: string, pagePath: string): string {
  const base = repoUrl.replace(/\/+$/, '');
  const relative = pagePath.replace(/^\/+/, '');
  return `${base}/blob/main/website/content/docs/${encodeURI(relative)}`;
}
