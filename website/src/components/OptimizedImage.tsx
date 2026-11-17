'use client';

import ExportedImage, { type ExportedImageProps } from 'next-image-export-optimizer';
import { BASE_PATH } from '@/lib/layout.shared';

export function OptimizedImage({ basePath = BASE_PATH, ...props }: ExportedImageProps) {
  return <ExportedImage basePath={basePath} {...props} />;
}
