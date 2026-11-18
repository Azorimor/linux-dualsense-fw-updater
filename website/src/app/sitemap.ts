// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import type { MetadataRoute } from 'next';
import { buildSitemapEntries } from '@/lib/llms-urls';
import { source } from '@/lib/source';

export const dynamic = 'force-static';
export const revalidate = false;

export default function sitemap(): MetadataRoute.Sitemap {
  return buildSitemapEntries(source.getPages().map((page) => page.url));
}
