// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import Link from 'next/link';
import { ShieldAlert, Cpu, Usb, BookOpen, Download, AlertTriangle } from 'lucide-react';
import { REPO_URL } from '@/lib/layout.shared';
import { OptimizedImage } from '@/components/OptimizedImage';
import screenshotInitial from '@/assets/screenshots/screenshot-0-initial.png';
import screenshotController from '@/assets/screenshots/screenshot-1-initial-with-controller.png';
import screenshotFirmware from '@/assets/screenshots/screenshot-2-prepare-new-firmware.png';
import screenshotReview from '@/assets/screenshots/screenshot-3-upgrade-overview-and-consent.png';

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

const screenshots = [
  {
    step: 'Step 1 — Initial Scan',
    title: 'Waiting for Wired USB Connection',
    caption:
      'Prompts you to connect your DualSense controller directly over a USB data cable and warns that updates cannot run over Bluetooth.',
    image: screenshotInitial,
    alt: 'Unofficial Linux DualSense Firmware Updater initial screen waiting for a wired USB controller connection',
  },
  {
    step: 'Step 1 — Controller Detected',
    title: 'Board Model & Firmware Inspection',
    caption:
      'Displays the detected controller name, board revision (BDM-050), shell colorway, current firmware version, and build timestamp.',
    image: screenshotController,
    alt: 'Unofficial Linux DualSense Firmware Updater displaying a detected DualSense BDM-050 controller over USB HID',
  },
  {
    step: 'Step 2 — Firmware Source',
    title: 'Official Sony Catalog or Local .bin',
    caption:
      'Downloads official firmware metadata for your mapped controller hardware target or lets you load a local .bin firmware image.',
    image: screenshotFirmware,
    alt: 'Unofficial Linux DualSense Firmware Updater firmware selection step with official Sony download and local .bin options',
  },
  {
    step: 'Step 3 — Review & Consent',
    title: 'Version Comparison & Safety Confirmation',
    caption:
      'Compares current and target versions, flags same-version or untested DualSense Edge targets, and requires explicit irreversibility consent.',
    image: screenshotReview,
    alt: 'Unofficial Linux DualSense Firmware Updater review and consent screen showing version comparison and safety notices',
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

      <section className="space-y-6">
        <div className="space-y-2 text-center">
          <h2 className="text-2xl font-bold tracking-tight text-fd-foreground sm:text-3xl">
            Guided KDE Kirigami Workflow
          </h2>
          <p className="mx-auto max-w-2xl text-sm text-fd-muted-foreground sm:text-base">
            Step-by-step screenshots of the Unofficial Linux DualSense Firmware
            Updater from USB detection through firmware selection and safety
            confirmation.
          </p>
        </div>

        <div className="grid gap-6 md:grid-cols-2">
          {screenshots.map((shot) => (
            <figure
              key={shot.title}
              className="flex flex-col overflow-hidden rounded-xl border border-fd-border bg-fd-card"
            >
              <div className="border-b border-fd-border bg-fd-secondary/40 p-3">
                <OptimizedImage
                  src={shot.image}
                  alt={shot.alt}
                  sizes="(max-width: 768px) 100vw, 50vw"
                  className="h-auto w-full rounded-lg"
                />
              </div>
              <figcaption className="flex flex-1 flex-col gap-1.5 p-5">
                <span className="text-xs font-medium uppercase tracking-wider text-fd-muted-foreground">
                  {shot.step}
                </span>
                <h3 className="text-base font-semibold text-fd-card-foreground">
                  {shot.title}
                </h3>
                <p className="text-sm leading-relaxed text-fd-muted-foreground">
                  {shot.caption}
                </p>
              </figcaption>
            </figure>
          ))}
        </div>
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
