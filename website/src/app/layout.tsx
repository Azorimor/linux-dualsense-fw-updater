// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import type { Metadata } from 'next';
import { RootProvider } from 'fumadocs-ui/provider/next';
import type { ReactNode } from 'react';
import { BASE_PATH } from '@/lib/layout.shared';
import './global.css';

export const metadata: Metadata = {
  metadataBase: new URL('https://azorimor.github.io'),
  title: {
    template: '%s | Unofficial Linux DualSense Firmware Updater',
    default: 'Unofficial Linux DualSense Firmware Updater',
  },
  description:
    'Unofficial, community-maintained Linux KDE Kirigami desktop utility for inspecting and updating PlayStation DualSense controller firmware over USB HID. Not affiliated with Sony Interactive Entertainment or PlayStation.',
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
