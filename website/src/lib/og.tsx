// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { ImageResponse } from 'next/og';

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
 * Render a 1200x630 PNG Open Graph card using `next/og` `ImageResponse` and the
 * existing project icon from `website/public/icon-192x192.png`.
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

  return new ImageResponse(
    (
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          width: '100%',
          height: '100%',
          padding: '56px 64px',
          background:
            'linear-gradient(135deg, #090d16 0%, #111827 58%, #0f172a 100%)',
          color: '#f8fafc',
          borderBottom: '12px solid #3b82f6',
        }}
      >
        <div
          style={{
            display: 'flex',
            flexDirection: 'row',
            alignItems: 'center',
            gap: '18px',
          }}
        >
          <img
            src={iconSrc}
            width={56}
            height={56}
            alt=""
            style={{ borderRadius: '12px' }}
          />
          <div
            style={{
              fontSize: '26px',
              fontWeight: 600,
              color: '#93c5fd',
            }}
          >
            {OG_SITE_NAME}
          </div>
        </div>

        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '20px',
          }}
        >
          <div
            style={{
              fontSize: '54px',
              fontWeight: 800,
              lineHeight: 1.15,
              color: '#f8fafc',
            }}
          >
            {title}
          </div>
          {description ? (
            <div
              style={{
                fontSize: '28px',
                lineHeight: 1.4,
                color: '#cbd5e1',
              }}
            >
              {description}
            </div>
          ) : null}
        </div>

        <div
          style={{
            display: 'flex',
            flexDirection: 'row',
            justifyContent: 'space-between',
            alignItems: 'center',
            borderTop: '1px solid rgba(148, 163, 184, 0.28)',
            paddingTop: '20px',
            fontSize: '22px',
          }}
        >
          <div style={{ color: '#94a3b8' }}>
            Linux • KDE Kirigami + Rust • USB HID
          </div>
          <div style={{ color: '#60a5fa' }}>
            azorimor.github.io/linux-dualsense-fw-updater
          </div>
        </div>
      </div>
    ),
    OG_SIZE,
  );
}
