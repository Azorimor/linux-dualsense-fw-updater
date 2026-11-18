// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import type { Metadata } from 'next';
import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { DownloadPanel } from '@/components/DownloadPanel';
import { fetchLatestRelease } from '@/lib/github-release';
import { BASE_PATH, baseOptions } from '@/lib/layout.shared';

export const dynamic = 'force-static';

export const metadata: Metadata = {
  title: 'Download',
  description:
    'Download pre-built Linux AppImage and tarball releases for the Unofficial Linux DualSense Firmware Updater after reviewing the safety disclaimer.',
  openGraph: {
    title: 'Download',
    description:
      'Download pre-built Linux AppImage and tarball releases for the Unofficial Linux DualSense Firmware Updater after reviewing the safety disclaimer.',
    type: 'website',
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
    title: 'Download',
    description:
      'Download pre-built Linux AppImage and tarball releases for the Unofficial Linux DualSense Firmware Updater after reviewing the safety disclaimer.',
    images: [
      {
        url: `${BASE_PATH}/og/download/image.png`,
        alt: 'Download Unofficial Linux DualSense Firmware Updater',
      },
    ],
  },
};

export default async function DownloadPage() {
  const release = await fetchLatestRelease();

  return (
    <HomeLayout {...baseOptions()}>
      <main className="flex-1">
        <DownloadPanel release={release} />
      </main>
    </HomeLayout>
  );
}
