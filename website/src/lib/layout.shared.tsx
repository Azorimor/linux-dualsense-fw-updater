// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

export const REPO_URL = 'https://github.com/Azorimor/linux-dualsense-fw-updater';
export const BASE_PATH = '/linux-dualsense-fw-updater';

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <span className="inline-flex items-center gap-2.5">
          <img
            src={`${BASE_PATH}/favicon.svg`}
            alt="Unofficial DualSense Updater logo"
            width={24}
            height={24}
            className="size-6 shrink-0 rounded-md"
          />
          <span>Unofficial DualSense Updater</span>
        </span>
      ),
    },
    githubUrl: REPO_URL,
    links: [
      {
        text: 'Documentation',
        url: '/docs',
        active: 'nested-url',
      },
      {
        text: 'Supported Hardware',
        url: '/docs/supported-hardware',
      },
      {
        text: 'Download',
        url: '/download',
      },
    ],
  };
}
