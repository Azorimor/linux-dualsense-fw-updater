// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { createOgImageResponse } from '@/lib/og';

export const dynamic = 'force-static';
export const revalidate = false;

export async function GET() {
  return createOgImageResponse({
    title: 'Privacy Policy',
    description:
      'Privacy policy for the Unofficial Linux DualSense Firmware Updater website and desktop app.',
  });
}
