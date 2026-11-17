// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const THIS_DIR = path.dirname(fileURLToPath(import.meta.url));
const WEBSITE_ROOT = path.resolve(THIS_DIR, '../..');
const REPO_ROOT = path.resolve(WEBSITE_ROOT, '..');
const PUBLIC_DIR = path.join(WEBSITE_ROOT, 'public');
const BASE_PATH = '/linux-dualsense-fw-updater';

const EXPECTED_PUBLIC_FILES = [
  'apple-touch-icon.png',
  'favicon-16x16.png',
  'favicon-32x32.png',
  'favicon-48x48.png',
  'favicon.svg',
  'icon-192x192.png',
  'icon-512x512.png',
  'site.webmanifest',
];

test('web brand assets are moved into website/public without duplicating assets/web', () => {
  for (const filename of EXPECTED_PUBLIC_FILES) {
    assert.equal(
      fs.existsSync(path.join(REPO_ROOT, 'assets/web', filename)),
      false,
      `assets/web/${filename} should not exist after moving assets to website/public`,
    );
    const filePath = path.join(PUBLIC_DIR, filename);
    assert.equal(
      fs.existsSync(filePath),
      true,
      `Expected ${filename} in website/public/`,
    );
  }
});

test('site.webmanifest is valid and uses BASE_PATH for all icon URLs', () => {
  const raw = fs.readFileSync(path.join(PUBLIC_DIR, 'site.webmanifest'), 'utf8');
  const manifest = JSON.parse(raw) as {
    name: string;
    short_name: string;
    start_url: string;
    scope: string;
    icons: Array<{ src: string; sizes: string; type: string }>;
  };

  assert.equal(manifest.start_url, `${BASE_PATH}/`);
  assert.equal(manifest.scope, `${BASE_PATH}/`);
  assert.ok(Array.isArray(manifest.icons) && manifest.icons.length >= 2);

  const expectedSizes = new Set(['192x192', '512x512']);
  for (const icon of manifest.icons) {
    assert.ok(
      icon.src.startsWith(`${BASE_PATH}/`),
      `Manifest icon src must start with ${BASE_PATH}/: ${icon.src}`,
    );
    const relativeAsset = icon.src.slice(`${BASE_PATH}/`.length);
    assert.equal(
      fs.existsSync(path.join(PUBLIC_DIR, relativeAsset)),
      true,
      `Manifest icon missing in website/public/: ${relativeAsset}`,
    );
    expectedSizes.delete(icon.sizes);
  }
  assert.equal(expectedSizes.size, 0, 'Missing 192x192 or 512x512 manifest icon');
});

test('layout.tsx and layout.shared.tsx wire logo, icons, and manifest with BASE_PATH', () => {
  const layoutShared = fs.readFileSync(
    path.join(WEBSITE_ROOT, 'src/lib/layout.shared.tsx'),
    'utf8',
  );
  assert.match(layoutShared, /export const BASE_PATH = '\/linux-dualsense-fw-updater'/);
  assert.match(layoutShared, /\$\{BASE_PATH\}\/favicon\.svg/);
  assert.match(layoutShared, /Unofficial DualSense Updater/);

  const rootLayout = fs.readFileSync(
    path.join(WEBSITE_ROOT, 'src/app/layout.tsx'),
    'utf8',
  );
  assert.match(rootLayout, /manifest:\s*`\$\{BASE_PATH\}\/site\.webmanifest`/);
  for (const iconFile of [
    'favicon.svg',
    'favicon-48x48.png',
    'favicon-32x32.png',
    'favicon-16x16.png',
    'apple-touch-icon.png',
  ]) {
    assert.ok(
      rootLayout.includes(`\${BASE_PATH}/${iconFile}`),
      `layout.tsx metadata.icons missing \${BASE_PATH}/${iconFile}`,
    );
  }
});

test('built static export HTML prefixes all favicon, manifest, and logo links with BASE_PATH', (t) => {
  const outDir = path.join(WEBSITE_ROOT, 'out');
  const indexHtmlPath = path.join(outDir, 'index.html');
  if (!fs.existsSync(indexHtmlPath)) {
    t.skip('website/out/index.html not built yet');
    return;
  }

  const html = fs.readFileSync(indexHtmlPath, 'utf8');
  const expectedRefs = [
    `${BASE_PATH}/site.webmanifest`,
    `${BASE_PATH}/favicon.svg`,
    `${BASE_PATH}/favicon-48x48.png`,
    `${BASE_PATH}/favicon-32x32.png`,
    `${BASE_PATH}/favicon-16x16.png`,
    `${BASE_PATH}/apple-touch-icon.png`,
  ];

  for (const ref of expectedRefs) {
    assert.ok(
      html.includes(`"${ref}"`),
      `Built out/index.html missing prefixed URL ${ref}`,
    );
    const relativeAsset = ref.slice(`${BASE_PATH}/`.length);
    assert.equal(
      fs.existsSync(path.join(outDir, relativeAsset)),
      true,
      `Built website/out/ missing ${relativeAsset}`,
    );
  }
});
