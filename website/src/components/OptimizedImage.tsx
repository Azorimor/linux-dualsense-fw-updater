// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

'use client';

import ExportedImage, { type ExportedImageProps } from 'next-image-export-optimizer';
import { BASE_PATH } from '@/lib/layout.shared';

export function OptimizedImage({ basePath = BASE_PATH, ...props }: ExportedImageProps) {
  return <ExportedImage basePath={basePath} {...props} />;
}
