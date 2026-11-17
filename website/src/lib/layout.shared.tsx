// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

export const REPO_URL = 'https://github.com/Azorimor/linux-dualsense-fw-updater';
export const BASE_PATH = '/linux-dualsense-fw-updater';

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: 'Unofficial DualSense Updater',
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
