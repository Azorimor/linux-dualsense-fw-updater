// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { generateOGImage } from 'fumadocs-ui/og';

export const OG_SITE_NAME = 'Unofficial Linux DualSense Firmware Updater';
export const OG_SIZE = {
  width: 1200,
  height: 630,
};
export const OG_CONTENT_TYPE = 'image/png';

const iconBase64Promise = readFile(
  join(process.cwd(), 'public', 'icon-192x192.png'),
  'base64',
);

/**
 * Render a 1200x630 PNG Open Graph card using `fumadocs-ui/og` and the existing
 * project icon from `website/public/icon-192x192.png`.
 */
export async function createOgImageResponse({
  title,
  description,
}: {
  title: string;
  description?: string;
}) {
  const iconBase64 = await iconBase64Promise;
  const iconSrc = `data:image/png;base64,${iconBase64}`;

  return generateOGImage({
    title,
    description,
    site: OG_SITE_NAME,
    primaryColor: 'rgba(59, 130, 246, 0.35)',
    primaryTextColor: 'rgb(96, 165, 250)',
    icon: (
      <img
        src={iconSrc}
        width={56}
        height={56}
        alt=""
        style={{ borderRadius: '12px' }}
      />
    ),
    ...OG_SIZE,
  });
}
