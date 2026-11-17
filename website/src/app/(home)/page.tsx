// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import Link from 'next/link';
import { ShieldAlert, Cpu, Usb, BookOpen, Download } from 'lucide-react';
import { OptimizedImage } from '@/components/OptimizedImage';
import { REPO_URL } from '@/lib/layout.shared';

const features = [
  {
    icon: Usb,
    title: 'Wired USB HID Inspection',
    description:
      'Reads Feature Report 0x20 and 0x81 directly over USB HID, displaying current firmware version, build date, board revision, and colorway.',
  },
  {
    icon: Cpu,
    title: 'Automatic Board Detection',
    description:
      'Maps BDM-020 through BDM-060R/M/X and DualSense Edge to Sony firmware targets (0004, 000B, 000E, 0044) and blocks unsupported BDM-010 flashes.',
  },
  {
    icon: ShieldAlert,
    title: 'Guided Safety & Recovery',
    description:
      'Monitors USB unplugs before flashing, enforces explicit irreversibility confirmation, and translates HID failures into plain-English recovery steps.',
  },
];

export default function HomePage() {
  return (
    <main className="mx-auto flex w-full max-w-6xl flex-1 flex-col gap-12 px-6 py-12 md:py-16">
      <section className="flex flex-col items-center gap-6 text-center">
        <span className="inline-flex items-center rounded-full border border-fd-border bg-fd-secondary px-3 py-1 text-xs font-medium text-fd-secondary-foreground">
          Unofficial Linux Desktop Utility &bull; KDE Kirigami + Rust
        </span>

        <h1 className="max-w-3xl text-4xl font-bold tracking-tight text-fd-foreground sm:text-5xl">
          Linux DualSense Firmware Updater
        </h1>

        <p className="max-w-2xl text-base text-fd-muted-foreground sm:text-lg">
          Inspect and update Sony PlayStation DualSense and DualSense Edge
          controller firmware directly on Linux over USB HID &mdash; without
          needing a Windows PC.
        </p>

        <div className="flex flex-wrap items-center justify-center gap-3 pt-2">
          <Link
            href="/docs"
            className="inline-flex items-center gap-2 rounded-lg bg-fd-primary px-5 py-2.5 text-sm font-medium text-fd-primary-foreground transition-opacity hover:opacity-90"
          >
            <BookOpen className="size-4" />
            Read Documentation
          </Link>
          <Link
            href="/docs/getting-started"
            className="inline-flex items-center gap-2 rounded-lg border border-fd-border bg-fd-card px-5 py-2.5 text-sm font-medium text-fd-card-foreground transition-colors hover:bg-fd-accent"
          >
            <Download className="size-4" />
            AppImage &amp; Setup
          </Link>
          <a
            href={REPO_URL}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-2 rounded-lg border border-fd-border px-5 py-2.5 text-sm font-medium text-fd-muted-foreground transition-colors hover:bg-fd-accent hover:text-fd-foreground"
          >
            GitHub Repository
          </a>
        </div>
      </section>

      <section className="overflow-hidden rounded-2xl border border-fd-border bg-fd-card p-3 shadow-sm">
        <OptimizedImage
          src="/images/dualsense-update-flow.png"
          alt="Linux DualSense Firmware Updater 4-step workflow and board-to-target mapping overview"
          width={1280}
          height={640}
          priority
          className="h-auto w-full rounded-xl"
        />
      </section>

      <section className="grid gap-6 md:grid-cols-3">
        {features.map((feature) => {
          const Icon = feature.icon;
          return (
            <div
              key={feature.title}
              className="flex flex-col gap-3 rounded-xl border border-fd-border bg-fd-card p-6"
            >
              <div className="inline-flex size-10 items-center justify-center rounded-lg bg-fd-primary/10 text-fd-primary">
                <Icon className="size-5" />
              </div>
              <h2 className="text-lg font-semibold text-fd-card-foreground">
                {feature.title}
              </h2>
              <p className="text-sm leading-relaxed text-fd-muted-foreground">
                {feature.description}
              </p>
            </div>
          );
        })}
      </section>

      <section className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-5 text-sm text-fd-foreground">
        <p className="font-semibold">Unofficial Tool Disclaimer</p>
        <p className="mt-1 text-fd-muted-foreground">
          This is not an official Sony application. Firmware upgrades are
          auto-committed by the controller even if verify and finalize steps do
          not run. Standard DualSense controllers are tested; DualSense Edge (
          <code>0044</code>) remains untested and is flagged before flashing.
        </p>
      </section>
    </main>
  );
}
