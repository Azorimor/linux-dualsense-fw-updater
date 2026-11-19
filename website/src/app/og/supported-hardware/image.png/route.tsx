// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { createOgImageResponse } from '@/lib/og';

export const dynamic = 'force-static';
export const revalidate = false;

export async function GET() {
  return createOgImageResponse({
    title: 'Supported Hardware',
    description:
      'Check if your PlayStation DualSense or DualSense Edge controller is supported for Linux firmware updates.',
  });
}
