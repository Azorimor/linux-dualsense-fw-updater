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
  toCanonicalPageUrl,
} from './llms-urls.ts';
import {
  HARD_MAX_DESCRIPTION_LENGTH,
  MIN_DESCRIPTION_LENGTH,
  RECOMMENDED_MAX_DESCRIPTION_LENGTH,
  SITE_NAME,
  buildBreadcrumbJsonLd,
  buildJsonLdGraph,
  buildTechArticleJsonLd,
  serializeJsonLd,
  validateMetaDescription,
} from './seo.ts';

const THIS_DIR = path.dirname(fileURLToPath(import.meta.url));
const WEBSITE_ROOT = path.resolve(THIS_DIR, '../..');
const OUT_DIR = path.join(WEBSITE_ROOT, 'out');
const PUBLIC_DIR = path.join(WEBSITE_ROOT, 'public');
const GOOGLE_VERIFICATION_FILE = 'googlef342c4b75d9721d6.html';

function collectHtmlPages(dir: string, rootDir = dir): string[] {
  const pages: string[] = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (entry.name === '404' || entry.name === '_not-found') {
        continue;
      }
      pages.push(...collectHtmlPages(path.join(dir, entry.name), rootDir));
    } else if (entry.isFile() && entry.name === 'index.html') {
      const rel = path.relative(rootDir, path.join(dir, entry.name));
      pages.push(rel.split(path.sep).join('/'));
    }
  }
  return pages.sort();
}

function decodeHtmlEntities(value: string): string {
  return value
    .replace(/&#x([0-9a-fA-F]+);/g, (_, hex: string) =>
      String.fromCodePoint(Number.parseInt(hex, 16)),
    )
    .replace(/&#(\d+);/g, (_, dec: string) =>
      String.fromCodePoint(Number.parseInt(dec, 10)),
    )
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&');
}

function parseHtmlAttributes(tagAttrs: string): Record<string, string> {
  const attrs: Record<string, string> = {};
  const attrRe = /([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*(?:"([^"]*)"|'([^']*)')/g;
  for (const match of tagAttrs.matchAll(attrRe)) {
    const key = match[1].toLowerCase();
    const rawValue = match[2] ?? match[3] ?? '';
    attrs[key] = decodeHtmlEntities(rawValue);
  }
  return attrs;
}

function getMetaContent(
  html: string,
  attrName: 'name' | 'property',
  attrValue: string,
): string | null {
  for (const match of html.matchAll(/<meta\b([^>]*)>/gi)) {
    const attrs = parseHtmlAttributes(match[1]);
    if (attrs[attrName]?.toLowerCase() === attrValue.toLowerCase()) {
      return attrs.content ?? null;
    }
  }
  return null;
}

function getCanonicalHref(html: string): string | null {
  for (const match of html.matchAll(/<link\b([^>]*)>/gi)) {
    const attrs = parseHtmlAttributes(match[1]);
    const relTokens = (attrs.rel ?? '').toLowerCase().split(/\s+/);
    if (relTokens.includes('canonical')) {
      return attrs.href ?? null;
    }
  }
  return null;
}

function extractJsonLdPayloads(html: string): unknown[] {
  const payloads: unknown[] = [];
  for (const match of html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)) {
    const attrs = parseHtmlAttributes(match[1]);
    if (attrs.type?.toLowerCase() !== 'application/ld+json') {
      continue;
    }
    const rawJson = match[2].trim();
    assert.ok(rawJson.length > 0, 'Empty <script type="application/ld+json"> tag');
    payloads.push(JSON.parse(rawJson));
  }
  return payloads;
}

function expectedCanonicalUrlForHtml(relPath: string): string {
  if (relPath === 'index.html') {
    return toCanonicalPageUrl('/');
  }
  const route = `/${relPath.replace(/\/index\.html$/, '')}`;
  return toCanonicalPageUrl(route);
}

test('serializeJsonLd escapes < as \\u003c and preserves round-trip JSON', () => {
  const payload = {
    '@context': 'https://schema.org',
    headline: 'DualSense </script><script>alert(1)</script>',
  };
  const serialized = serializeJsonLd(payload);
  assert.equal(serialized.includes('<'), false);
  assert.ok(serialized.includes('\\u003c/script>'));
  assert.deepEqual(JSON.parse(serialized), payload);
  assert.equal(serializeJsonLd(undefined), 'null');
});

test('buildBreadcrumbJsonLd formats Schema.org BreadcrumbList with 1-based positions', () => {
  const result = buildBreadcrumbJsonLd([
    { name: 'Home', url: 'https://example.com/' },
    { name: 'Documentation', url: 'https://example.com/docs/' },
    { name: 'Getting Started', url: 'https://example.com/docs/getting-started/' },
  ]);

  assert.deepEqual(result, {
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: [
      {
        '@type': 'ListItem',
        position: 1,
        name: 'Home',
        item: 'https://example.com/',
      },
      {
        '@type': 'ListItem',
        position: 2,
        name: 'Documentation',
        item: 'https://example.com/docs/',
      },
      {
        '@type': 'ListItem',
        position: 3,
        name: 'Getting Started',
        item: 'https://example.com/docs/getting-started/',
      },
    ],
  });
  assert.deepEqual(buildBreadcrumbJsonLd([]).itemListElement, []);
});

test('buildTechArticleJsonLd emits Schema.org TechArticle metadata', () => {
  const withImage = buildTechArticleJsonLd({
    title: 'Overview',
    description: 'Documentation overview for Linux DualSense updater.',
    url: 'https://azorimor.github.io/linux-dualsense-fw-updater/docs/',
    imageUrl: 'https://azorimor.github.io/linux-dualsense-fw-updater/og/docs/image.png',
  });

  assert.equal(withImage['@context'], 'https://schema.org');
  assert.equal(withImage['@type'], 'TechArticle');
  assert.equal(withImage.headline, 'Overview');
  assert.equal(
    withImage.image,
    'https://azorimor.github.io/linux-dualsense-fw-updater/og/docs/image.png',
  );
  assert.equal(withImage.publisher.name, SITE_NAME);
  assert.deepEqual(withImage.mainEntityOfPage, {
    '@type': 'WebPage',
    '@id': 'https://azorimor.github.io/linux-dualsense-fw-updater/docs/',
  });

  const withoutImage = buildTechArticleJsonLd({
    title: 'Licenses',
    description: 'Open-source license details for DualSense Firmware Updater.',
    url: 'https://azorimor.github.io/linux-dualsense-fw-updater/docs/licenses/',
  });
  assert.equal('image' in withoutImage, false);

  const graph = buildJsonLdGraph([
    withImage,
    buildBreadcrumbJsonLd([
      { name: 'Home', url: 'https://azorimor.github.io/linux-dualsense-fw-updater/' },
    ]),
  ]);
  assert.equal(graph['@context'], 'https://schema.org');
  assert.equal(graph['@graph'].length, 2);
  assert.equal('@context' in graph['@graph'][0], false);
  assert.equal('@context' in graph['@graph'][1], false);
});

test('validateMetaDescription enforces 40..200 character bounds', () => {
  assert.equal(RECOMMENDED_MAX_DESCRIPTION_LENGTH, 160);
  assert.equal(HARD_MAX_DESCRIPTION_LENGTH, 200);

  const empty = validateMetaDescription('   ');
  assert.equal(empty.valid, false);
  assert.equal(empty.length, 0);
  assert.ok(empty.reason);

  const tooShort = validateMetaDescription('a'.repeat(MIN_DESCRIPTION_LENGTH - 1));
  assert.equal(tooShort.valid, false);
  assert.equal(tooShort.length, 39);
  assert.ok(tooShort.reason);

  const minBoundary = validateMetaDescription('a'.repeat(MIN_DESCRIPTION_LENGTH));
  assert.deepEqual(minBoundary, { valid: true, length: 40 });

  const recommended = validateMetaDescription(
    'Unofficial open-source Linux app to check and update PlayStation DualSense and DualSense Edge controller firmware over USB. Not affiliated with Sony.',
  );
  assert.deepEqual(recommended, { valid: true, length: 149 });

  const maxBoundary = validateMetaDescription('a'.repeat(HARD_MAX_DESCRIPTION_LENGTH));
  assert.deepEqual(maxBoundary, { valid: true, length: 200 });

  const tooLong = validateMetaDescription('a'.repeat(HARD_MAX_DESCRIPTION_LENGTH + 1));
  assert.equal(tooLong.valid, false);
  assert.equal(tooLong.length, 201);
  assert.ok(tooLong.reason);
});

test('built static export satisfies SEO metadata, JSON-LD, verification, and copy audit', (t) => {
  const indexHtmlPath = path.join(OUT_DIR, 'index.html');
  if (!fs.existsSync(indexHtmlPath)) {
    t.skip('website/out/index.html not built yet');
    return;
  }

  const pages = collectHtmlPages(OUT_DIR);
  assert.ok(pages.length > 0, 'Expected at least one HTML page in website/out/');

  assert.equal(
    fs.existsSync(path.join(OUT_DIR, 'supported-hardware/index.html')),
    true,
    'Expected out/supported-hardware/index.html to exist',
  );
  assert.equal(
    fs.existsSync(path.join(OUT_DIR, 'docs/supported-hardware/index.html')),
    false,
    'Legacy out/docs/supported-hardware/index.html must not exist',
  );

  const publicVerificationPath = path.join(PUBLIC_DIR, GOOGLE_VERIFICATION_FILE);
  const outVerificationPath = path.join(OUT_DIR, GOOGLE_VERIFICATION_FILE);
  assert.equal(
    fs.existsSync(publicVerificationPath),
    true,
    `Missing website/public/${GOOGLE_VERIFICATION_FILE}`,
  );
  assert.equal(
    fs.existsSync(outVerificationPath),
    true,
    `Missing website/out/${GOOGLE_VERIFICATION_FILE}`,
  );
  assert.equal(
    Buffer.compare(
      fs.readFileSync(outVerificationPath),
      fs.readFileSync(publicVerificationPath),
    ),
    0,
    `website/out/${GOOGLE_VERIFICATION_FILE} must match website/public/${GOOGLE_VERIFICATION_FILE} byte-for-byte`,
  );

  const seenTitles = new Map<string, string>();
  const canonicalPrefix = `${SITE_ORIGIN}${SITE_BASE_PATH}/`;

  for (const relPath of pages) {
    const fullHtml = fs.readFileSync(path.join(OUT_DIR, relPath), 'utf8');
    const htmlNoScripts = fullHtml.replace(
      /<script\b[^>]*>[\s\S]*?<\/script>/gi,
      '',
    );

    assert.match(
      htmlNoScripts,
      /<html\b[^>]*\blang=(?:"en"|'en')(?=[\s>])/i,
      `out/${relPath} missing <html lang="en">`,
    );

    const h1Matches = htmlNoScripts.match(/<h1\b[^>]*>[\s\S]*?<\/h1>/gi) ?? [];
    assert.equal(
      h1Matches.length,
      1,
      `Expected exactly 1 <h1> on out/${relPath}, found ${h1Matches.length}`,
    );

    const titleMatch = htmlNoScripts.match(/<title\b[^>]*>([\s\S]*?)<\/title>/i);
    assert.ok(titleMatch, `out/${relPath} missing <title>`);
    const title = decodeHtmlEntities(titleMatch[1]).trim();
    assert.ok(title.length > 0, `out/${relPath} has empty <title>`);
    const existingOwner = seenTitles.get(title);
    assert.equal(
      existingOwner,
      undefined,
      `Duplicate <title> "${title}" found on out/${relPath} and out/${existingOwner}`,
    );
    seenTitles.set(title, relPath);

    const metaDescription = getMetaContent(htmlNoScripts, 'name', 'description');
    assert.ok(
      metaDescription !== null,
      `out/${relPath} missing <meta name="description">`,
    );
    const descValidation = validateMetaDescription(metaDescription);
    assert.equal(
      descValidation.valid,
      true,
      `out/${relPath} invalid meta description: ${descValidation.reason ?? ''}`,
    );
    assert.ok(
      descValidation.length <= RECOMMENDED_MAX_DESCRIPTION_LENGTH,
      `out/${relPath} meta description (${descValidation.length} chars) exceeds recommended ${RECOMMENDED_MAX_DESCRIPTION_LENGTH} chars`,
    );

    const ogDescription = getMetaContent(htmlNoScripts, 'property', 'og:description');
    assert.ok(
      ogDescription !== null && ogDescription.trim().length > 0,
      `out/${relPath} missing <meta property="og:description">`,
    );
    assert.ok(
      ogDescription.trim().length <= HARD_MAX_DESCRIPTION_LENGTH,
      `out/${relPath} og:description (${ogDescription.trim().length} chars) exceeds ${HARD_MAX_DESCRIPTION_LENGTH}`,
    );

    const twitterDescription = getMetaContent(
      htmlNoScripts,
      'name',
      'twitter:description',
    );
    assert.ok(
      twitterDescription !== null && twitterDescription.trim().length > 0,
      `out/${relPath} missing <meta name="twitter:description">`,
    );
    assert.ok(
      twitterDescription.trim().length <= HARD_MAX_DESCRIPTION_LENGTH,
      `out/${relPath} twitter:description (${twitterDescription.trim().length} chars) exceeds ${HARD_MAX_DESCRIPTION_LENGTH}`,
    );

    const canonicalHref = getCanonicalHref(htmlNoScripts);
    assert.ok(
      canonicalHref !== null,
      `out/${relPath} missing <link rel="canonical">`,
    );
    assert.ok(
      canonicalHref.startsWith(canonicalPrefix),
      `out/${relPath} canonical URL must start with ${canonicalPrefix}, got ${canonicalHref}`,
    );
    assert.ok(
      canonicalHref.endsWith('/'),
      `out/${relPath} canonical URL must end with '/', got ${canonicalHref}`,
    );
    assert.equal(
      canonicalHref,
      expectedCanonicalUrlForHtml(relPath),
      `out/${relPath} canonical URL mismatch`,
    );

    const ogUrl = getMetaContent(htmlNoScripts, 'property', 'og:url');
    assert.equal(
      ogUrl,
      canonicalHref,
      `out/${relPath} og:url (${ogUrl}) must match canonical (${canonicalHref})`,
    );

    const jsonLdPayloads = extractJsonLdPayloads(fullHtml);
    assert.ok(
      jsonLdPayloads.length >= 1,
      `out/${relPath} missing <script type="application/ld+json">`,
    );
    const hasSchemaOrgContext = jsonLdPayloads.some(
      (payload) =>
        typeof payload === 'object' &&
        payload !== null &&
        (payload as Record<string, unknown>)['@context'] === 'https://schema.org',
    );
    assert.equal(
      hasSchemaOrgContext,
      true,
      `out/${relPath} JSON-LD must declare "@context": "https://schema.org"`,
    );
  }

  const homeHtml = fs
    .readFileSync(indexHtmlPath, 'utf8')
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '');
  for (const forbidden of [
    'KDE Kirigami + Rust',
    'GitHub Repository',
    'BDM-020',
    'BDM-060',
    'Feature Report 0x20',
  ]) {
    assert.equal(
      homeHtml.includes(forbidden),
      false,
      `out/index.html must not contain "${forbidden}"`,
    );
  }

  const downloadHtml = fs
    .readFileSync(path.join(OUT_DIR, 'download/index.html'), 'utf8')
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '');
  assert.ok(
    downloadHtml.includes('Portable Archive (.tar.gz)'),
    'out/download/index.html missing "Portable Archive (.tar.gz)"',
  );
  assert.ok(
    downloadHtml.includes('Recommended'),
    'out/download/index.html must mark Portable Archive (.tar.gz) as Recommended',
  );
});
