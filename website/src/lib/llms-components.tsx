// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { asMarkdown, md } from 'fumadocs-core/server';
import type { ReactNode } from 'react';

const CALLOUT_LABELS: Record<string, string> = {
  info: 'INFO',
  warn: 'WARNING',
  warning: 'WARNING',
  error: 'CAUTION',
  success: 'TIP',
  idea: 'NOTE',
};

/**
 * Render `<Callout>` MDX blocks as GitHub-style Markdown alert blockquotes when
 * `fumadocs-core` serializes processed Markdown for LLMs.
 */
export function MarkdownCallout({
  type = 'info',
  title,
  children,
}: {
  type?: string;
  title?: ReactNode;
  children?: ReactNode;
}) {
  if (!asMarkdown()) {
    return null;
  }

  const label = CALLOUT_LABELS[type] ?? type.toUpperCase();
  return md.linePrefix('> ')`[!${label}]${title ? ` ${title}` : ''}\n${children}`;
}

export const llmsMdxComponents = {
  Callout: MarkdownCallout,
};
