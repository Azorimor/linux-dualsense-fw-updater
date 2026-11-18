// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { notFound } from 'next/navigation';
import { getDocsOgImageSegments } from '@/lib/llms-urls';
import { createOgImageResponse } from '@/lib/og';
import { source } from '@/lib/source';

export const dynamic = 'force-static';
export const revalidate = false;
export const dynamicParams = false;

export async function GET(
  _req: Request,
  { params }: { params: Promise<{ slug: string[] }> },
) {
  const { slug } = await params;
  const page = source.getPage(slug.slice(0, -1));
  if (!page) {
    notFound();
  }

  return createOgImageResponse({
    title: page.data.title,
    description: page.data.description,
  });
}

export function generateStaticParams() {
  return source.getPages().map((page) => ({
    slug: getDocsOgImageSegments(page.slugs),
  }));
}
