// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  remarkStripIncludedH1,
  type MdastNode,
} from './remark-strip-included-h1.ts';

const THIS_DIR = path.dirname(fileURLToPath(import.meta.url));
const WEBSITE_ROOT = path.resolve(THIS_DIR, '../..');
const REPO_ROOT = path.resolve(WEBSITE_ROOT, '..');

test('remarkStripIncludedH1 removes matching leading H1 inside included root nodes', () => {
  const includedRoot: MdastNode = {
    type: 'root',
    children: [
      {
        type: 'heading',
        depth: 1,
        children: [{ type: 'text', value: 'Changelog' }],
      },
      {
        type: 'paragraph',
        children: [{ type: 'text', value: 'All notable changes...' }],
      },
      {
        type: 'heading',
        depth: 2,
        children: [{ type: 'text', value: '[0.0.0] - 2026-11-17' }],
      },
    ],
  };

  const tree: MdastNode = {
    type: 'root',
    children: [includedRoot],
  };

  remarkStripIncludedH1()(tree, {
    data: { frontmatter: { title: 'Changelog' } },
  });

  assert.equal(includedRoot.children?.length, 2);
  assert.equal(includedRoot.children?.[0].type, 'paragraph');
  assert.equal(includedRoot.children?.[1].type, 'heading');
  assert.equal(includedRoot.children?.[1].depth, 2);
});

test('remarkStripIncludedH1 preserves non-matching headings and H2+', () => {
  const includedRoot: MdastNode = {
    type: 'root',
    children: [
      {
        type: 'heading',
        depth: 1,
        children: [{ type: 'text', value: 'Different Heading' }],
      },
    ],
  };

  const tree: MdastNode = {
    type: 'root',
    children: [includedRoot],
  };

  remarkStripIncludedH1()(tree, {
    data: { frontmatter: { title: 'Changelog' } },
  });

  assert.equal(includedRoot.children?.length, 1);

  const h2Root: MdastNode = {
    type: 'root',
    children: [
      {
        type: 'heading',
        depth: 2,
        children: [{ type: 'text', value: 'Changelog' }],
      },
    ],
  };
  remarkStripIncludedH1()(
    { type: 'root', children: [h2Root] },
    { data: { frontmatter: { title: 'Changelog' } } },
  );
  assert.equal(h2Root.children?.length, 1);
});

test('remarkStripIncludedH1 handles missing title or empty tree safely', () => {
  const emptyIncluded: MdastNode = { type: 'root', children: [] };
  const tree: MdastNode = { type: 'root', children: [emptyIncluded] };

  remarkStripIncludedH1()(tree, {});
  remarkStripIncludedH1()(tree, { data: { frontmatter: { title: '   ' } } });
  remarkStripIncludedH1()(tree, { data: { frontmatter: { title: 'Changelog' } } });

  assert.equal(emptyIncluded.children?.length, 0);
});

test('changelog.mdx includes root CHANGELOG.md without duplicating it in website/', () => {
  assert.equal(
    fs.existsSync(path.join(REPO_ROOT, 'CHANGELOG.md')),
    true,
    'Root CHANGELOG.md must remain at repository root',
  );
  assert.equal(
    fs.existsSync(path.join(WEBSITE_ROOT, 'CHANGELOG.md')),
    false,
    'CHANGELOG.md must not be copied into website/',
  );
  assert.equal(
    fs.existsSync(path.join(WEBSITE_ROOT, 'content/docs/CHANGELOG.md')),
    false,
    'CHANGELOG.md must not be copied into website/content/docs/',
  );

  const mdx = fs.readFileSync(
    path.join(WEBSITE_ROOT, 'content/docs/changelog.mdx'),
    'utf8',
  );
  assert.match(mdx, /<include>\.\.\/\.\.\/\.\.\/CHANGELOG\.md<\/include>/);

  const meta = JSON.parse(
    fs.readFileSync(path.join(WEBSITE_ROOT, 'content/docs/meta.json'), 'utf8'),
  ) as { pages: string[] };
  assert.ok(
    meta.pages.includes('changelog'),
    'website/content/docs/meta.json must include "changelog"',
  );
});

test('built static export renders changelog page with a single H1 and root CHANGELOG.md entries', (t) => {
  const changelogHtmlPath = path.join(
    WEBSITE_ROOT,
    'out/docs/changelog/index.html',
  );
  if (!fs.existsSync(changelogHtmlPath)) {
    t.skip('website/out/docs/changelog/index.html not built yet');
    return;
  }

  const html = fs.readFileSync(changelogHtmlPath, 'utf8');
  const h1Matches = html.match(/<h1\b[^>]*>.*?<\/h1>/g) ?? [];
  assert.equal(
    h1Matches.length,
    1,
    `Expected exactly 1 <h1> on changelog page, found ${h1Matches.length}`,
  );
  assert.ok(
    html.includes('All notable changes to this project will be documented in this file.'),
    'Built changelog page missing intro text from root CHANGELOG.md',
  );
  assert.ok(
    html.includes('[0.0.0] - 2026-11-17'),
    'Built changelog page missing release heading from root CHANGELOG.md',
  );

  const searchIndexPath = path.join(WEBSITE_ROOT, 'out/search-index.json');
  assert.equal(fs.existsSync(searchIndexPath), true);
  const searchIndex = fs.readFileSync(searchIndexPath, 'utf8');
  assert.ok(
    searchIndex.includes('/docs/changelog'),
    'Static search-index.json missing /docs/changelog entry',
  );
});
