// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { docs } from 'fumadocs-mdx:collections/server';
import { llms, loader } from 'fumadocs-core/source';
import { llmsMdxComponents } from './llms-components';
import { toStaticLlmsUrl } from './llms-urls';

export const source = loader({
  baseUrl: '/docs',
  source: docs.toFumadocsSource(),
});

export const docsLlms = llms(source, {
  renderPage: async (page) => {
    const processed = await page.data.getText('processed', {
      components: llmsMdxComponents,
    });
    return `# ${page.data.title} (${page.url})\n\n${processed}`;
  },
  renderUrl: (item) => toStaticLlmsUrl(item.url),
});
