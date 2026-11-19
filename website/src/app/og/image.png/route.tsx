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
      'Unofficial open-source Linux app to check and update PlayStation DualSense and DualSense Edge controller firmware over USB.',
  });
}
