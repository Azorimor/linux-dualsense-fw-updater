// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { source } from '@/lib/source';
import {
  DocsBody,
  DocsDescription,
  DocsPage,
  DocsTitle,
  MarkdownCopyButton,
  PageFooter,
  ViewOptionsPopover,
} from 'fumadocs-ui/layouts/docs/page';
import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { SiteFooter } from '@/components/SiteFooter';
import { REPO_URL } from '@/lib/layout.shared';
import {
  SITE_ORIGIN,
  getDocGitHubUrl,
  getDocsOgImageUrl,
  getPageActionMarkdownUrl,
  getStaticPageMarkdownUrl,
  toCanonicalPageUrl,
} from '@/lib/llms-urls';
import {
  SITE_NAME,
  buildBreadcrumbJsonLd,
  buildJsonLdGraph,
  buildTechArticleJsonLd,
  serializeJsonLd,
} from '@/lib/seo';
import { getMDXComponents } from '@/mdx-components';

export default async function Page(props: {
  params: Promise<{ slug?: string[] }>;
}) {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  const MDX = page.data.body;
  const markdownUrl = getPageActionMarkdownUrl(page.url);
  const githubUrl = getDocGitHubUrl(REPO_URL, page.path);
  const canonicalUrl = toCanonicalPageUrl(page.url);
  const ogImageUrl = `${SITE_ORIGIN}${getDocsOgImageUrl(page.slugs)}`;

  const breadcrumbItems = [
    { name: 'Home', url: toCanonicalPageUrl('/') },
    { name: 'Documentation', url: toCanonicalPageUrl('/docs') },
  ];
  if (page.slugs.length > 0) {
    breadcrumbItems.push({ name: page.data.title, url: canonicalUrl });
  }

  const jsonLd = buildJsonLdGraph([
    buildTechArticleJsonLd({
      title: page.data.title,
      description: page.data.description ?? '',
      url: canonicalUrl,
      imageUrl: ogImageUrl,
    }),
    buildBreadcrumbJsonLd(breadcrumbItems),
  ]);

  return (
    <DocsPage toc={page.data.toc} full={page.data.full} footer={{ enabled: false }}>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: serializeJsonLd(jsonLd) }}
      />
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>
      <div className="flex flex-row gap-2 items-center border-b pt-2 pb-6">
        <MarkdownCopyButton markdownUrl={markdownUrl} />
        <ViewOptionsPopover markdownUrl={markdownUrl} githubUrl={githubUrl} />
      </div>
      <DocsBody>
        <MDX components={getMDXComponents()} />
      </DocsBody>
      <PageFooter />
      <SiteFooter variant="docs" />
    </DocsPage>
  );
}

export async function generateStaticParams() {
  return source.generateParams();
}

export async function generateMetadata(props: {
  params: Promise<{ slug?: string[] }>;
}): Promise<Metadata> {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  const canonicalUrl = toCanonicalPageUrl(page.url);
  const ogImageUrl = getDocsOgImageUrl(page.slugs);

  return {
    title: page.data.title,
    description: page.data.description,
    alternates: {
      canonical: canonicalUrl,
      types: {
        'text/markdown': getStaticPageMarkdownUrl(page.url),
      },
    },
    openGraph: {
      title: page.data.title,
      description: page.data.description,
      type: 'article',
      siteName: SITE_NAME,
      locale: 'en_US',
      url: canonicalUrl,
      images: [
        {
          url: ogImageUrl,
          width: 1200,
          height: 630,
          alt: page.data.title,
        },
      ],
    },
    twitter: {
      card: 'summary_large_image',
      title: page.data.title,
      description: page.data.description,
      images: [
        {
          url: ogImageUrl,
          alt: page.data.title,
        },
      ],
    },
  };
}
