// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { OG_SITE_NAME, createOgImageResponse } from '@/lib/og';

export const dynamic = 'force-static';
export const revalidate = false;

export async function GET() {
  return createOgImageResponse({
    title: OG_SITE_NAME,
    description:
      'Unofficial, community-maintained Linux KDE Kirigami desktop utility for inspecting and updating PlayStation DualSense firmware over USB HID.',
  });
}
