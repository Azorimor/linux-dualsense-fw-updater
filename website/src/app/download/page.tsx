// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import type { Metadata } from 'next';
import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { DownloadPanel } from '@/components/DownloadPanel';
import { SiteFooter } from '@/components/SiteFooter';
import { fetchLatestRelease } from '@/lib/github-release';
import { BASE_PATH, baseOptions } from '@/lib/layout.shared';
import { toCanonicalPageUrl } from '@/lib/llms-urls';

export const dynamic = 'force-static';

const DOWNLOAD_TITLE = 'Download';
const DOWNLOAD_DESCRIPTION =
  'Download portable Linux (.tar.gz) archives for x86_64 and aarch64 (ARM64) of the Unofficial Linux DualSense Firmware Updater.';
const DOWNLOAD_CANONICAL_URL = toCanonicalPageUrl('/download');

export const metadata: Metadata = {
  title: DOWNLOAD_TITLE,
  description: DOWNLOAD_DESCRIPTION,
  alternates: {
    canonical: DOWNLOAD_CANONICAL_URL,
  },
  openGraph: {
    title: DOWNLOAD_TITLE,
    description: DOWNLOAD_DESCRIPTION,
    type: 'website',
    siteName: 'Unofficial Linux DualSense Firmware Updater',
    locale: 'en_US',
    url: DOWNLOAD_CANONICAL_URL,
    images: [
      {
        url: `${BASE_PATH}/og/download/image.png`,
        width: 1200,
        height: 630,
        alt: 'Download Unofficial Linux DualSense Firmware Updater',
      },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    title: DOWNLOAD_TITLE,
    description: DOWNLOAD_DESCRIPTION,
    images: [
      {
        url: `${BASE_PATH}/og/download/image.png`,
        alt: 'Download Unofficial Linux DualSense Firmware Updater',
      },
    ],
  },
};

const downloadJsonLd = {
  '@context': 'https://schema.org',
  '@graph': [
    {
      '@type': 'WebPage',
      name: 'Download Unofficial Linux DualSense Firmware Updater',
      description: DOWNLOAD_DESCRIPTION,
      url: DOWNLOAD_CANONICAL_URL,
      isPartOf: {
        '@type': 'WebSite',
        name: 'Unofficial Linux DualSense Firmware Updater',
        url: toCanonicalPageUrl('/'),
      },
    },
    {
      '@type': 'BreadcrumbList',
      itemListElement: [
        {
          '@type': 'ListItem',
          position: 1,
          name: 'Home',
          item: toCanonicalPageUrl('/'),
        },
        {
          '@type': 'ListItem',
          position: 2,
          name: 'Download',
          item: DOWNLOAD_CANONICAL_URL,
        },
      ],
    },
  ],
};

export default async function DownloadPage() {
  const release = await fetchLatestRelease();

  return (
    <HomeLayout {...baseOptions()}>
      <main className="flex-1">
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify(downloadJsonLd).replace(/</g, '\\u003c'),
          }}
        />
        <DownloadPanel release={release} />
      </main>
      <SiteFooter />
    </HomeLayout>
  );
}
