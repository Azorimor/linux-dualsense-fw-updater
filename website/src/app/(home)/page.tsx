// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import Link from 'next/link';
import { ShieldAlert, Cpu, Usb, BookOpen, Download, AlertTriangle } from 'lucide-react';
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
      'Maps BDM-020 through BDM-060R/M/X and DualSense Edge to firmware targets (0004, 000B, 000E, 0044) and blocks unsupported BDM-010 flashes.',
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
    <main className="mx-auto flex w-full max-w-6xl flex-1 flex-col gap-10 px-6 py-12 md:py-16">
      <section className="flex flex-col items-center gap-6 text-center">
        <span className="inline-flex items-center rounded-full border border-fd-border bg-fd-secondary px-3 py-1 text-xs font-medium text-fd-secondary-foreground">
          Unofficial Community Project &bull; KDE Kirigami + Rust
        </span>

        <h1 className="max-w-3xl text-4xl font-bold tracking-tight text-fd-foreground sm:text-5xl">
          Unofficial Linux DualSense Firmware Updater
        </h1>

        <p className="max-w-2xl text-base text-fd-muted-foreground sm:text-lg">
          An unofficial, community-maintained Linux utility for inspecting and
          updating DualSense and DualSense Edge controller firmware over USB HID.
          This project has no affiliation, endorsement, or relation to Sony
          Interactive Entertainment or PlayStation.
        </p>

        <div className="flex flex-wrap items-center justify-center gap-3 pt-2">
          <Link
            href="/download"
            className="inline-flex items-center gap-2 rounded-lg bg-fd-primary px-5 py-2.5 text-sm font-medium text-fd-primary-foreground transition-opacity hover:opacity-90"
          >
            <Download className="size-4" />
            Download
          </Link>
          <Link
            href="/docs"
            className="inline-flex items-center gap-2 rounded-lg border border-fd-border bg-fd-card px-5 py-2.5 text-sm font-medium text-fd-card-foreground transition-colors hover:bg-fd-accent"
          >
            <BookOpen className="size-4" />
            Read Documentation
          </Link>
          <a
            href={REPO_URL}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-2 rounded-lg border border-fd-border bg-fd-card px-5 py-2.5 text-sm font-medium text-fd-muted-foreground transition-colors hover:bg-fd-accent hover:text-fd-foreground"
          >
            GitHub Repository
          </a>
        </div>
      </section>

      <section className="grid gap-6 md:grid-cols-3">
        {features.map((feature) => {
          const Icon = feature.icon;
          return (
            <div
              key={feature.title}
              className="flex flex-col gap-3 rounded-xl border border-fd-border bg-fd-card p-6"
            >
              <div className="inline-flex size-10 items-center justify-center rounded-lg border border-fd-border bg-fd-secondary text-fd-foreground">
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

      <section className="rounded-xl border border-amber-500/40 bg-amber-500/10 p-6 text-sm text-fd-foreground">
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-0.5 size-5 shrink-0 text-amber-500" />
          <div className="space-y-2">
            <p className="font-semibold">
              Unofficial Community Project &mdash; No Sony Affiliation
            </p>
            <p className="leading-relaxed text-fd-muted-foreground">
              This software is an unofficial, community-maintained project with
              no affiliation, endorsement, or relation to Sony Interactive
              Entertainment or PlayStation. Flashing firmware can permanently
              damage or brick your controller: upgrades are auto-committed by
              the controller even if verify and finalize steps do not run. Only
              use this tool if you have no access to Sony&apos;s official Windows
              updater or a PlayStation 5 console. Standard DualSense controllers
              are tested; DualSense Edge (<code>0044</code>) remains untested
              and is flagged before flashing.
            </p>
          </div>
        </div>
      </section>
    </main>
  );
}
