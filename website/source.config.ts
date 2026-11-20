// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import path from 'node:path';
import { pageSchema } from 'fumadocs-core/source/schema';
import {
  defineCollections,
  defineConfig,
  defineDocs,
} from 'fumadocs-mdx/config';
import { remarkStripIncludedH1 } from './src/lib/remark-strip-included-h1';

export const docs = defineDocs({
  dir: 'content/docs',
  docs: {
    postprocess: {
      includeProcessedMarkdown: true,
    },
  },
});

export const pages = defineCollections({
  type: 'doc',
  dir: 'content/pages',
  schema: pageSchema,
});

export default defineConfig({
  plugins: [
    {
      name: 'filter-outside-root-dependencies',
      doc: {
        vfile(file) {
          const compiler = file.data._compiler as
            | { addDependency?: (dep: string) => void }
            | undefined;
          if (!compiler?.addDependency) {
            return;
          }

          const root = path.resolve(this.core.root);
          const originalAddDependency = compiler.addDependency.bind(compiler);
          compiler.addDependency = (dep: string) => {
            const resolved = path.resolve(dep);
            if (resolved === root || resolved.startsWith(`${root}${path.sep}`)) {
              originalAddDependency(dep);
            }
          };
        },
      },
    },
  ],
  mdxOptions: {
    preset: 'fumadocs',
    remarkPlugins: (plugins) => [remarkStripIncludedH1, ...plugins],
  },
});
