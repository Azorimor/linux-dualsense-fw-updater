// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { pages } from 'fumadocs-mdx:collections/server';
import { HomeLayout } from 'fumadocs-ui/layouts/home';
import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { SiteFooter } from '@/components/SiteFooter';
import { BASE_PATH, baseOptions } from '@/lib/layout.shared';
import { toCanonicalPageUrl } from '@/lib/llms-urls';
import {
  SITE_NAME,
  buildBreadcrumbJsonLd,
  buildJsonLdGraph,
  serializeJsonLd,
} from '@/lib/seo';
import { getMDXComponents } from '@/mdx-components';

export const dynamic = 'force-static';

const CANONICAL_URL = toCanonicalPageUrl('/privacy');
const OG_IMAGE_URL = `${BASE_PATH}/og/privacy/image.png`;

function getPrivacyPage() {
  return pages.find((entry) => entry.info.path === 'privacy.mdx') ?? null;
}

export function generateMetadata(): Metadata {
  const page = getPrivacyPage();
  if (!page) {
    notFound();
  }

  const title = page.title;
  const description = page.description ?? '';

  return {
    title,
    description,
    alternates: {
      canonical: CANONICAL_URL,
    },
    openGraph: {
      type: 'website',
      siteName: SITE_NAME,
      locale: 'en_US',
      url: CANONICAL_URL,
      title,
      description,
      images: [
        {
          url: OG_IMAGE_URL,
          width: 1200,
          height: 630,
          alt: `${title} — ${SITE_NAME}`,
        },
      ],
    },
    twitter: {
      card: 'summary_large_image',
      title,
      description,
      images: [
        {
          url: OG_IMAGE_URL,
          alt: `${title} — ${SITE_NAME}`,
        },
      ],
    },
  };
}

export default function PrivacyPage() {
  const page = getPrivacyPage();
  if (!page) {
    notFound();
  }

  const MDX = page.body;
  const jsonLd = buildJsonLdGraph([
    {
      '@type': 'WebPage',
      '@id': CANONICAL_URL,
      url: CANONICAL_URL,
      name: page.title,
      description: page.description ?? '',
      isPartOf: {
        '@type': 'WebSite',
        url: toCanonicalPageUrl('/'),
        name: SITE_NAME,
      },
    },
    buildBreadcrumbJsonLd([
      { name: 'Home', url: toCanonicalPageUrl('/') },
      { name: page.title, url: CANONICAL_URL },
    ]),
  ]);

  return (
    <HomeLayout {...baseOptions()}>
      <main className="flex-1">
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: serializeJsonLd(jsonLd) }}
        />

        <article className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-6 py-12 md:py-16">
          <header className="space-y-3 border-b border-fd-border pb-6">
            <h1 className="text-3xl font-bold tracking-tight text-fd-foreground sm:text-4xl">
              {page.title}
            </h1>
            {page.description ? (
              <p className="text-base text-fd-muted-foreground">
                {page.description}
              </p>
            ) : null}
          </header>

          <div className="prose max-w-none">
            <MDX components={getMDXComponents()} />
          </div>
        </article>
      </main>
      <SiteFooter />
    </HomeLayout>
  );
}
