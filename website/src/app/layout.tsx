// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import type { Metadata } from 'next';
import { RootProvider } from 'fumadocs-ui/provider/next';
import type { ReactNode } from 'react';
import { BASE_PATH } from '@/lib/layout.shared';
import { toCanonicalPageUrl } from '@/lib/llms-urls';
import { SITE_NAME } from '@/lib/seo';
import './global.css';

const ROOT_DESCRIPTION =
  'Unofficial open-source Linux app to check and update PlayStation DualSense and DualSense Edge controller firmware over USB. Not affiliated with Sony.';

export const metadata: Metadata = {
  metadataBase: new URL('https://azorimor.github.io'),
  title: {
    template: `%s | ${SITE_NAME}`,
    default: SITE_NAME,
  },
  description: ROOT_DESCRIPTION,
  applicationName: SITE_NAME,
  category: 'utilities',
  authors: [{ name: 'Azorimor', url: 'https://github.com/Azorimor' }],
  creator: 'Azorimor',
  keywords: [
    'DualSense firmware updater Linux',
    'PlayStation 5 controller firmware Linux',
    'DualSense Edge firmware Linux',
    'PS5 controller update Linux',
    'DualSense USB update',
  ],
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      'max-image-preview': 'large',
      'max-snippet': -1,
    },
  },
  openGraph: {
    type: 'website',
    siteName: SITE_NAME,
    locale: 'en_US',
    url: toCanonicalPageUrl('/'),
    title: SITE_NAME,
    description: ROOT_DESCRIPTION,
    images: [
      {
        url: `${BASE_PATH}/og/image.png`,
        width: 1200,
        height: 630,
        alt: SITE_NAME,
      },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    title: SITE_NAME,
    description: ROOT_DESCRIPTION,
    images: [
      {
        url: `${BASE_PATH}/og/image.png`,
        alt: SITE_NAME,
      },
    ],
  },
  manifest: `${BASE_PATH}/site.webmanifest`,
  icons: {
    icon: [
      { url: `${BASE_PATH}/favicon.svg`, type: 'image/svg+xml' },
      { url: `${BASE_PATH}/favicon-48x48.png`, sizes: '48x48', type: 'image/png' },
      { url: `${BASE_PATH}/favicon-32x32.png`, sizes: '32x32', type: 'image/png' },
      { url: `${BASE_PATH}/favicon-16x16.png`, sizes: '16x16', type: 'image/png' },
    ],
    shortcut: [`${BASE_PATH}/favicon-32x32.png`],
    apple: [
      { url: `${BASE_PATH}/apple-touch-icon.png`, sizes: '180x180', type: 'image/png' },
    ],
  },
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="flex min-h-screen flex-col">
        <RootProvider
          search={{
            options: {
              type: 'static',
              api: `${BASE_PATH}/search-index.json`,
            },
          }}
        >
          {children}
        </RootProvider>
      </body>
    </html>
  );
}
