// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

export const SITE_ORIGIN = 'https://azorimor.github.io';
export const SITE_BASE_PATH = '/linux-dualsense-fw-updater';

/**
 * Build a full canonical URL for an HTML page under GitHub Pages `basePath`
 * matching Next.js `trailingSlash: true`.
 */
export function toCanonicalPageUrl(
  routePath: string,
  origin = SITE_ORIGIN,
  basePath = SITE_BASE_PATH,
): string {
  const cleanOrigin = origin.replace(/\/+$/, '');
  const cleanBase = basePath.replace(/\/+$/, '');
  const trimmed = routePath.trim().replace(/^\/+|\/+$/g, '');
  return trimmed
    ? `${cleanOrigin}${cleanBase}/${trimmed}/`
    : `${cleanOrigin}${cleanBase}/`;
}

/**
 * Build a full URL for a root static asset (such as `/sitemap.xml` or
 * `/robots.txt`) under GitHub Pages `basePath`.
 */
export function toSiteAssetUrl(
  assetPath: string,
  origin = SITE_ORIGIN,
  basePath = SITE_BASE_PATH,
): string {
  const cleanOrigin = origin.replace(/\/+$/, '');
  const cleanBase = basePath.replace(/\/+$/, '');
  const cleanAsset = assetPath.trim().replace(/^\/+/, '');
  return `${cleanOrigin}${cleanBase}/${cleanAsset}`;
}

export interface SitemapEntry {
  url: string;
  changeFrequency: 'weekly';
  priority: number;
}

/**
 * Build deterministic sitemap entries for the home page, download page, and all
 * Fumadocs documentation pages.
 */
export function buildSitemapEntries(
  docUrls: readonly string[],
  origin = SITE_ORIGIN,
  basePath = SITE_BASE_PATH,
): SitemapEntry[] {
  const entries: SitemapEntry[] = [
    {
      url: toCanonicalPageUrl('/', origin, basePath),
      changeFrequency: 'weekly',
      priority: 1,
    },
    {
      url: toCanonicalPageUrl('/download', origin, basePath),
      changeFrequency: 'weekly',
      priority: 0.9,
    },
  ];

  const seen = new Set(entries.map((entry) => entry.url));
  for (const docUrl of docUrls) {
    const canonical = toCanonicalPageUrl(docUrl, origin, basePath);
    if (seen.has(canonical)) {
      continue;
    }
    seen.add(canonical);
    const isDocsRoot = docUrl.replace(/\/+$/, '') === '/docs';
    entries.push({
      url: canonical,
      changeFrequency: 'weekly',
      priority: isDocsRoot ? 0.9 : 0.8,
    });
  }

  return entries;
}

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
