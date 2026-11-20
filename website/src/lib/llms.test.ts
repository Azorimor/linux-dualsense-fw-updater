// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  SITE_BASE_PATH,
  SITE_ORIGIN,
  buildSitemapEntries,
  getDocGitHubUrl,
  getDocsOgImageSegments,
  getDocsOgImageUrl,
  getMarkdownPath,
  getPageActionMarkdownUrl,
  getStaticPageMarkdownUrl,
  toCanonicalPageUrl,
  toSiteAssetUrl,
  toStaticLlmsUrl,
} from './llms-urls.ts';

const THIS_DIR = path.dirname(fileURLToPath(import.meta.url));
const WEBSITE_ROOT = path.resolve(THIS_DIR, '../..');
const OUT_DIR = path.join(WEBSITE_ROOT, 'out');
const ORIGIN = 'https://azorimor.github.io';

test('getMarkdownPath maps root, index, and nested doc URLs', () => {
  assert.equal(getMarkdownPath('/'), '/index.md');
  assert.equal(getMarkdownPath(''), '/index.md');
  assert.equal(getMarkdownPath('/docs'), '/docs.md');
  assert.equal(getMarkdownPath('/docs/'), '/docs.md');
  assert.equal(getMarkdownPath('/docs/getting-started'), '/docs/getting-started.md');
});

test('toStaticLlmsUrl prefixes static markdown/text artifacts with BASE_PATH and /llms.mdx', () => {
  assert.equal(
    toStaticLlmsUrl('/docs.md'),
    `${SITE_BASE_PATH}/llms.mdx/docs.md`,
  );
  assert.equal(
    toStaticLlmsUrl('/docs/getting-started.md'),
    `${SITE_BASE_PATH}/llms.mdx/docs/getting-started.md`,
  );
  assert.equal(
    toStaticLlmsUrl('/docs/llms-full.txt'),
    `${SITE_BASE_PATH}/llms.mdx/docs/llms-full.txt`,
  );
  assert.equal(
    toStaticLlmsUrl('/docs/drafts'),
    `${SITE_BASE_PATH}/docs/drafts`,
  );
  assert.equal(
    toStaticLlmsUrl('https://example.com/external.md'),
    'https://example.com/external.md',
  );
});

test('getStaticPageMarkdownUrl, getPageActionMarkdownUrl, and getDocGitHubUrl construct expected links', () => {
  assert.equal(getPageActionMarkdownUrl('/docs'), '/llms.mdx/docs.md');
  assert.equal(
    getPageActionMarkdownUrl('/docs/changelog'),
    '/llms.mdx/docs/changelog.md',
  );
  assert.equal(
    getStaticPageMarkdownUrl('/docs'),
    `${SITE_BASE_PATH}/llms.mdx/docs.md`,
  );
  assert.equal(
    getStaticPageMarkdownUrl('/docs/changelog'),
    `${SITE_BASE_PATH}/llms.mdx/docs/changelog.md`,
  );
  assert.equal(
    getDocGitHubUrl('https://github.com/Azorimor/linux-dualsense-fw-updater', 'changelog.mdx'),
    'https://github.com/Azorimor/linux-dualsense-fw-updater/blob/main/website/content/docs/changelog.mdx',
  );
});

test('toCanonicalPageUrl, toSiteAssetUrl, and buildSitemapEntries produce trailing-slash GitHub Pages URLs', () => {
  assert.equal(toCanonicalPageUrl('/'), `${SITE_ORIGIN}${SITE_BASE_PATH}/`);
  assert.equal(
    toCanonicalPageUrl('/download'),
    `${SITE_ORIGIN}${SITE_BASE_PATH}/download/`,
  );
  assert.equal(
    toCanonicalPageUrl('/docs/getting-started'),
    `${SITE_ORIGIN}${SITE_BASE_PATH}/docs/getting-started/`,
  );
  assert.equal(
    toSiteAssetUrl('/sitemap.xml'),
    `${SITE_ORIGIN}${SITE_BASE_PATH}/sitemap.xml`,
  );

  const entries = buildSitemapEntries(['/docs', '/docs/getting-started', '/docs']);
  assert.deepEqual(
    entries.map((entry) => [entry.url, entry.priority]),
    [
      [`${SITE_ORIGIN}${SITE_BASE_PATH}/`, 1],
      [`${SITE_ORIGIN}${SITE_BASE_PATH}/download/`, 0.9],
      [`${SITE_ORIGIN}${SITE_BASE_PATH}/supported-hardware/`, 0.9],
      [`${SITE_ORIGIN}${SITE_BASE_PATH}/privacy/`, 0.5],
      [`${SITE_ORIGIN}${SITE_BASE_PATH}/docs/`, 0.9],
      [`${SITE_ORIGIN}${SITE_BASE_PATH}/docs/getting-started/`, 0.8],
    ],
  );
});

test('getDocsOgImageSegments and getDocsOgImageUrl map doc slugs to static PNG routes', () => {
  assert.deepEqual(getDocsOgImageSegments([]), ['image.png']);
  assert.deepEqual(getDocsOgImageSegments(['getting-started']), [
    'getting-started',
    'image.png',
  ]);
  assert.equal(
    getDocsOgImageUrl([]),
    `${SITE_BASE_PATH}/og/docs/image.png`,
  );
  assert.equal(
    getDocsOgImageUrl(['getting-started']),
    `${SITE_BASE_PATH}/og/docs/getting-started/image.png`,
  );
});

test('built static export generates llms.txt, llms-full.txt, and per-page markdown files', (t) => {
  const llmsTxtPath = path.join(OUT_DIR, 'llms.txt');
  if (!fs.existsSync(llmsTxtPath)) {
    t.skip('website/out/llms.txt not built yet');
    return;
  }

  const llmsTxt = fs.readFileSync(llmsTxtPath, 'utf8');
  const expectedDocs = [
    {
      mdRel: 'llms.mdx/docs.md',
      htmlRel: 'docs/index.html',
      ogRel: 'og/docs/image.png',
    },
    {
      mdRel: 'llms.mdx/docs/getting-started.md',
      htmlRel: 'docs/getting-started/index.html',
      ogRel: 'og/docs/getting-started/image.png',
    },
    {
      mdRel: 'llms.mdx/docs/safety-and-troubleshooting.md',
      htmlRel: 'docs/safety-and-troubleshooting/index.html',
      ogRel: 'og/docs/safety-and-troubleshooting/image.png',
    },
    {
      mdRel: 'llms.mdx/docs/changelog.md',
      htmlRel: 'docs/changelog/index.html',
      ogRel: 'og/docs/changelog/image.png',
    },
    {
      mdRel: 'llms.mdx/docs/licenses.md',
      htmlRel: 'docs/licenses/index.html',
      ogRel: 'og/docs/licenses/image.png',
    },
  ];

  for (const { mdRel, htmlRel, ogRel } of expectedDocs) {
    const prefixedUrl = `${SITE_BASE_PATH}/${mdRel}`;
    assert.ok(
      llmsTxt.includes(`(${prefixedUrl})`),
      `llms.txt missing link to ${prefixedUrl}`,
    );
    assert.equal(
      fs.existsSync(path.join(OUT_DIR, mdRel)),
      true,
      `Missing static markdown file out/${mdRel}`,
    );

    const html = fs.readFileSync(path.join(OUT_DIR, htmlRel), 'utf8');
    assert.ok(
      html.includes(`type="text/markdown" href="${ORIGIN}${prefixedUrl}"`),
      `out/${htmlRel} missing alternate text/markdown <link> for ${prefixedUrl}`,
    );
    assert.ok(
      html.includes('Copy Markdown'),
      `out/${htmlRel} missing MarkdownCopyButton`,
    );
    const unprefixedMd = `/${mdRel}`;
    assert.ok(
      html.includes(`"markdownUrl":"${unprefixedMd}"`) ||
        html.includes(`\\"markdownUrl\\":\\"${unprefixedMd}\\"`),
      `out/${htmlRel} should pass unprefixed markdownUrl ${unprefixedMd} to page actions`,
    );
    assert.equal(
      html.includes(`${SITE_BASE_PATH}${SITE_BASE_PATH}`),
      false,
      `out/${htmlRel} must not contain doubled BASE_PATH`,
    );

    const ogFullUrl = `${ORIGIN}${SITE_BASE_PATH}/${ogRel}`;
    assert.ok(
      html.includes(`property="og:image" content="${ogFullUrl}"`),
      `out/${htmlRel} missing og:image ${ogFullUrl}`,
    );
    assert.ok(
      html.includes(`name="twitter:image" content="${ogFullUrl}"`),
      `out/${htmlRel} missing twitter:image ${ogFullUrl}`,
    );
    const ogPngPath = path.join(OUT_DIR, ogRel);
    assert.equal(fs.existsSync(ogPngPath), true, `Missing out/${ogRel}`);
    assert.deepEqual(
      fs.readFileSync(ogPngPath).subarray(0, 4),
      Buffer.from([0x89, 0x50, 0x4e, 0x47]),
      `out/${ogRel} is not a valid PNG`,
    );
  }

  for (const { htmlRel, ogRel } of [
    { htmlRel: 'index.html', ogRel: 'og/image.png' },
    { htmlRel: 'download/index.html', ogRel: 'og/download/image.png' },
    {
      htmlRel: 'supported-hardware/index.html',
      ogRel: 'og/supported-hardware/image.png',
    },
    {
      htmlRel: 'privacy/index.html',
      ogRel: 'og/privacy/image.png',
    },
  ]) {
    const html = fs.readFileSync(path.join(OUT_DIR, htmlRel), 'utf8');
    const ogFullUrl = `${ORIGIN}${SITE_BASE_PATH}/${ogRel}`;
    assert.ok(
      html.includes(`property="og:image" content="${ogFullUrl}"`),
      `out/${htmlRel} missing og:image ${ogFullUrl}`,
    );
    assert.ok(
      html.includes(`name="twitter:image" content="${ogFullUrl}"`),
      `out/${htmlRel} missing twitter:image ${ogFullUrl}`,
    );
    const ogPngPath = path.join(OUT_DIR, ogRel);
    assert.equal(fs.existsSync(ogPngPath), true, `Missing out/${ogRel}`);
    assert.deepEqual(
      fs.readFileSync(ogPngPath).subarray(0, 4),
      Buffer.from([0x89, 0x50, 0x4e, 0x47]),
      `out/${ogRel} is not a valid PNG`,
    );
  }

  const llmsFullPath = path.join(OUT_DIR, 'llms-full.txt');
  assert.equal(fs.existsSync(llmsFullPath), true, 'Missing out/llms-full.txt');
  const llmsFull = fs.readFileSync(llmsFullPath, 'utf8');
  assert.equal(
    llmsFull.includes('<Callout'),
    false,
    'llms-full.txt should render Callout components as Markdown blockquotes, not raw JSX',
  );
  assert.ok(
    llmsFull.includes('> [!WARNING]'),
    'llms-full.txt should render warning callouts as GitHub-style alert blockquotes',
  );

  const changelogMd = fs.readFileSync(
    path.join(OUT_DIR, 'llms.mdx/docs/changelog.md'),
    'utf8',
  );
  const changelogH1s = changelogMd.match(/^# .+$/gm) ?? [];
  assert.equal(
    changelogH1s.length,
    1,
    `Expected exactly 1 H1 in changelog.md, found ${changelogH1s.length}`,
  );

  const robotsPath = path.join(OUT_DIR, 'robots.txt');
  assert.equal(fs.existsSync(robotsPath), true, 'Missing out/robots.txt');
  const robotsTxt = fs.readFileSync(robotsPath, 'utf8');
  assert.match(robotsTxt, /User-Agent:\s*\*/i);
  assert.match(robotsTxt, /Allow:\s*\//i);
  assert.ok(
    robotsTxt.includes(`Sitemap: ${ORIGIN}${SITE_BASE_PATH}/sitemap.xml`),
    'robots.txt missing Sitemap URL',
  );

  const sitemapPath = path.join(OUT_DIR, 'sitemap.xml');
  assert.equal(fs.existsSync(sitemapPath), true, 'Missing out/sitemap.xml');
  const sitemapXml = fs.readFileSync(sitemapPath, 'utf8');
  for (const routePath of [
    '/',
    '/download/',
    '/supported-hardware/',
    '/privacy/',
    '/docs/',
    '/docs/getting-started/',
    '/docs/safety-and-troubleshooting/',
    '/docs/changelog/',
    '/docs/licenses/',
  ]) {
    const loc = `<loc>${ORIGIN}${SITE_BASE_PATH}${routePath}</loc>`;
    assert.ok(sitemapXml.includes(loc), `sitemap.xml missing ${loc}`);
  }
});
