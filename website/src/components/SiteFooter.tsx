// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import Link from 'next/link';
import { REPO_URL } from '@/lib/layout.shared';

export interface SiteFooterProps {
  variant?: 'default' | 'docs';
}

export function SiteFooter({ variant = 'default' }: SiteFooterProps) {
  const isDocs = variant === 'docs';

  const content = (
    <div
      className={
        isDocs
          ? 'flex flex-col gap-3 text-xs text-fd-muted-foreground sm:flex-row sm:items-center sm:justify-between'
          : 'mx-auto flex w-full max-w-6xl flex-col gap-3 px-6 py-6 text-xs text-fd-muted-foreground sm:flex-row sm:items-center sm:justify-between'
      }
    >
      <p>
        Unofficial Linux DualSense Firmware Updater &middot; GPL-3.0-or-later
        &middot; Not affiliated with Sony or PlayStation.
      </p>
      <nav aria-label="Footer navigation" className="flex flex-wrap items-center gap-4">
        <Link
          href="/privacy"
          className="transition-colors hover:text-fd-foreground"
        >
          Privacy Policy
        </Link>
        <Link
          href="/docs/licenses"
          className="transition-colors hover:text-fd-foreground"
        >
          Licenses
        </Link>
        <a
          href={REPO_URL}
          target="_blank"
          rel="noreferrer"
          className="transition-colors hover:text-fd-foreground"
        >
          GitHub
        </a>
      </nav>
    </div>
  );

  if (isDocs) {
    return (
      <footer
        aria-label="Site footer"
        className="mt-6 border-t border-fd-border pt-6"
      >
        {content}
      </footer>
    );
  }

  return (
    <footer
      aria-label="Site footer"
      className="mt-auto border-t border-fd-border bg-fd-background"
    >
      {content}
    </footer>
  );
}
