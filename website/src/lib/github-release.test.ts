// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import assert from 'node:assert/strict';
import test from 'node:test';
import {
  buildGitHubHeaders,
  classifyReleaseAsset,
  detectReleaseAssetArch,
  fetchLatestRelease,
  formatBytes,
  formatPublishedDate,
  parseGitHubRelease,
  parseLatestFromReleasesList,
  parseSha256Digest,
  partitionReleaseAssets,
} from './github-release.ts';

test('classifyReleaseAsset identifies portable tarballs and checksums while ignoring AppImage', () => {
  assert.equal(
    classifyReleaseAsset('linux-dualsense-fw-updater-v0.1.0-x86_64.tar.gz'),
    'tarball',
  );
  assert.equal(
    classifyReleaseAsset('linux-dualsense-fw-updater-v0.1.0-aarch64.tar.gz'),
    'tarball',
  );
  assert.equal(classifyReleaseAsset('SHA256SUMS.txt'), 'checksums');
  assert.equal(
    classifyReleaseAsset('linux-dualsense-fw-updater-v0.1.0-x86_64.AppImage'),
    null,
  );
  assert.equal(classifyReleaseAsset('source.zip'), null);
});

test('detectReleaseAssetArch identifies x86_64 and aarch64 architectures', () => {
  assert.equal(
    detectReleaseAssetArch('linux-dualsense-fw-updater-v0.1.0-x86_64.tar.gz'),
    'x86_64',
  );
  assert.equal(
    detectReleaseAssetArch('linux-dualsense-fw-updater-v0.1.0-amd64.tar.gz'),
    'x86_64',
  );
  assert.equal(
    detectReleaseAssetArch('linux-dualsense-fw-updater-v0.1.0-aarch64.tar.gz'),
    'aarch64',
  );
  assert.equal(
    detectReleaseAssetArch('linux-dualsense-fw-updater-v0.1.0-arm64.tar.gz'),
    'aarch64',
  );
  assert.equal(
    detectReleaseAssetArch('linux-dualsense-fw-updater-v0.1.0.tar.gz'),
    undefined,
  );
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

test('parseGitHubRelease extracts sorted x86_64, aarch64, and checksums assets while skipping drafts and AppImage', () => {
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
        size: 230,
        digest:
          'sha256:25fda8e6f57a9f19581df1b467533474ee8c2bf4f6a364e0c1e87d68d13bcc31',
        browser_download_url:
          'https://github.com/Azorimor/linux-dualsense-fw-updater/releases/download/v0.1.0/SHA256SUMS.txt',
      },
      {
        name: 'notes.pdf',
        size: 1000,
        browser_download_url: 'https://example.com/notes.pdf',
      },
      {
        name: 'linux-dualsense-fw-updater-v0.1.0-aarch64.tar.gz',
        size: 9 * 1024 * 1024,
        digest:
          'sha256:24e3b06957841ae1ccf9c303b49dedc16df572b64ab5dd7840f2b45beec5ce94',
        browser_download_url:
          'https://github.com/Azorimor/linux-dualsense-fw-updater/releases/download/v0.1.0/linux-dualsense-fw-updater-v0.1.0-aarch64.tar.gz',
      },
      {
        name: 'linux-dualsense-fw-updater-v0.1.0-x86_64.tar.gz',
        size: 10 * 1024 * 1024,
        digest:
          'sha256:141365fb377eb6b1e183514bd6493b8b78246e2bf32eb0bb4654f170ba226f08',
        browser_download_url:
          'https://github.com/Azorimor/linux-dualsense-fw-updater/releases/download/v0.1.0/linux-dualsense-fw-updater-v0.1.0-x86_64.tar.gz',
      },
      {
        name: 'linux-dualsense-fw-updater-v0.1.0-x86_64.AppImage',
        size: 45 * 1024 * 1024,
        browser_download_url:
          'https://github.com/Azorimor/linux-dualsense-fw-updater/releases/download/v0.1.0/linux-dualsense-fw-updater-v0.1.0-x86_64.AppImage',
      },
    ],
  });

  assert.ok(parsed);
  assert.equal(parsed.tagName, 'v0.1.0');
  assert.equal(parsed.name, 'v0.1.0 Initial Release');
  assert.equal(parsed.assets.length, 3);
  assert.deepEqual(
    parsed.assets.map((a) => [a.kind, a.arch]),
    [
      ['tarball', 'x86_64'],
      ['tarball', 'aarch64'],
      ['checksums', undefined],
    ],
  );
  assert.equal(parsed.assets[0].formattedSize, '10.0 MiB');
  assert.equal(
    parsed.assets[0].sha256,
    '141365fb377eb6b1e183514bd6493b8b78246e2bf32eb0bb4654f170ba226f08',
  );
  assert.equal(parsed.assets[1].formattedSize, '9.0 MiB');
  assert.equal(
    parsed.assets[1].sha256,
    '24e3b06957841ae1ccf9c303b49dedc16df572b64ab5dd7840f2b45beec5ce94',
  );

  const partitioned = partitionReleaseAssets(parsed.assets);
  assert.equal(partitioned.x86_64?.arch, 'x86_64');
  assert.equal(partitioned.aarch64?.arch, 'aarch64');
  assert.equal(partitioned.checksums?.kind, 'checksums');
  assert.equal(partitioned.otherBuilds.length, 0);

  assert.equal(parseSha256Digest('sha256:not-hex'), undefined);
  assert.equal(
    parseLatestFromReleasesList([
      {
        tag_name: 'v0.2.0-rc1',
        draft: false,
        prerelease: true,
        html_url: 'https://github.com/example/rc1',
        assets: [],
      },
      {
        tag_name: 'v0.1.0',
        draft: false,
        prerelease: false,
        html_url: 'https://github.com/example/v0.1.0',
        assets: [],
      },
    ])?.tagName,
    'v0.1.0',
  );
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
