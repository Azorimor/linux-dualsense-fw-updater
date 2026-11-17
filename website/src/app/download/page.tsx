// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import type { Metadata } from 'next';
import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { DownloadPanel } from '@/components/DownloadPanel';
import { fetchLatestRelease } from '@/lib/github-release';
import { baseOptions } from '@/lib/layout.shared';

export const metadata: Metadata = {
  title: 'Download',
  description:
    'Download pre-built Linux AppImage and tarball releases for the Unofficial Linux DualSense Firmware Updater after reviewing the safety disclaimer.',
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
