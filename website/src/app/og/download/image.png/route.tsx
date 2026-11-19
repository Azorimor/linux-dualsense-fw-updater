// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { createOgImageResponse } from '@/lib/og';

export const dynamic = 'force-static';
export const revalidate = false;

export async function GET() {
  return createOgImageResponse({
    title: 'Download',
    description:
      'Download the recommended portable Linux archive (.tar.gz) or AppImage after reviewing the safety disclaimer.',
  });
}
