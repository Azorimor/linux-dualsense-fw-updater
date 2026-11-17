// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import assert from 'node:assert/strict';
import test from 'node:test';
import {
  buildGitHubHeaders,
  classifyReleaseAsset,
  fetchLatestRelease,
  formatBytes,
  formatPublishedDate,
  parseGitHubRelease,
} from './github-release.ts';

test('classifyReleaseAsset identifies AppImage, tarball, and checksums', () => {
  assert.equal(
    classifyReleaseAsset('linux-dualsense-fw-updater-v0.1.0-x86_64.AppImage'),
    'appimage',
  );
  assert.equal(
    classifyReleaseAsset('linux-dualsense-fw-updater-v0.1.0-x86_64.tar.gz'),
    'tarball',
  );
  assert.equal(classifyReleaseAsset('SHA256SUMS.txt'), 'checksums');
  assert.equal(classifyReleaseAsset('source.zip'), null);
});

test('formatBytes formats binary sizes cleanly', () => {
  assert.equal(formatBytes(0), '0 B');
  assert.equal(formatBytes(512), '512 B');
  assert.equal(formatBytes(1536), '1.5 KiB');
  assert.equal(formatBytes(12 * 1024 * 1024), '12.0 MiB');
});

test('formatPublishedDate formats ISO UTC dates and handles missing/invalid', () => {
  assert.equal(formatPublishedDate('2026-11-17T10:30:00Z'), 'Nov 17, 2026');
  assert.equal(formatPublishedDate(''), 'Unknown date');
  assert.equal(formatPublishedDate('not-a-date'), 'not-a-date');
});

test('buildGitHubHeaders attaches Bearer token only when non-empty', () => {
  const withoutToken = buildGitHubHeaders(undefined);
  assert.equal(withoutToken.Accept, 'application/vnd.github+json');
  assert.equal(withoutToken.Authorization, undefined);

  const blankToken = buildGitHubHeaders('   ');
  assert.equal(blankToken.Authorization, undefined);

  const withToken = buildGitHubHeaders('ghp_example123');
  assert.equal(withToken.Authorization, 'Bearer ghp_example123');
});

test('parseGitHubRelease extracts sorted matching assets and skips drafts/non-matching', () => {
  const parsed = parseGitHubRelease({
    tag_name: 'v0.1.0',
    name: 'v0.1.0 Initial Release',
    draft: false,
    published_at: '2026-11-17T12:00:00Z',
    html_url:
      'https://github.com/Azorimor/linux-dualsense-fw-updater/releases/tag/v0.1.0',
    assets: [
      {
        name: 'SHA256SUMS.txt',
        size: 256,
        browser_download_url:
          'https://github.com/Azorimor/linux-dualsense-fw-updater/releases/download/v0.1.0/SHA256SUMS.txt',
      },
      {
        name: 'notes.pdf',
        size: 1000,
        browser_download_url: 'https://example.com/notes.pdf',
      },
      {
        name: 'dualsense-updater-0.1.0-x86_64.tar.gz',
        size: 10 * 1024 * 1024,
        browser_download_url:
          'https://github.com/Azorimor/linux-dualsense-fw-updater/releases/download/v0.1.0/dualsense-updater-0.1.0-x86_64.tar.gz',
      },
      {
        name: 'dualsense-updater-0.1.0-x86_64.AppImage',
        size: 25 * 1024 * 1024,
        browser_download_url:
          'https://github.com/Azorimor/linux-dualsense-fw-updater/releases/download/v0.1.0/dualsense-updater-0.1.0-x86_64.AppImage',
      },
    ],
  });

  assert.ok(parsed);
  assert.equal(parsed.tagName, 'v0.1.0');
  assert.equal(parsed.name, 'v0.1.0 Initial Release');
  assert.equal(parsed.assets.length, 3);
  assert.deepEqual(
    parsed.assets.map((a) => a.kind),
    ['appimage', 'tarball', 'checksums'],
  );
  assert.equal(parsed.assets[0].formattedSize, '25.0 MiB');
  assert.equal(
    parseGitHubRelease({
      tag_name: 'v0.1.0',
      draft: true,
      html_url: 'https://github.com/example',
    }),
    null,
  );
});

test('fetchLatestRelease gracefully returns null on 404 or network error', async () => {
  const notFoundResult = await fetchLatestRelease({
    token: 'test-token',
    fetchImpl: async (_input, init) => {
      const headers = init?.headers as Record<string, string>;
      assert.equal(headers.Authorization, 'Bearer test-token');
      return new Response('Not Found', { status: 404 });
    },
  });
  assert.equal(notFoundResult, null);

  const networkErrResult = await fetchLatestRelease({
    fetchImpl: async () => {
      throw new Error('ECONNREFUSED');
    },
  });
  assert.equal(networkErrResult, null);
});
