// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import type { Metadata } from 'next';
import Link from 'next/link';
import {
  ShieldAlert,
  Cpu,
  Usb,
  BookOpen,
  Download,
  AlertTriangle,
} from 'lucide-react';
import { BASE_PATH, REPO_URL } from '@/lib/layout.shared';
import { toCanonicalPageUrl } from '@/lib/llms-urls';
import { OptimizedImage } from '@/components/OptimizedImage';
import screenshotInitial from '@/assets/screenshots/screenshot-0-initial.png';
import screenshotController from '@/assets/screenshots/screenshot-1-initial-with-controller.png';
import screenshotFirmware from '@/assets/screenshots/screenshot-2-prepare-new-firmware.png';
import screenshotReview from '@/assets/screenshots/screenshot-3-upgrade-overview-and-consent.png';

const HOME_TITLE = 'Unofficial Linux DualSense Firmware Updater';
const HOME_DESCRIPTION =
  'Unofficial open-source Linux app to check and update PlayStation DualSense and DualSense Edge controller firmware over USB. Not affiliated with Sony.';
const HOME_CANONICAL_URL = toCanonicalPageUrl('/');

export const metadata: Metadata = {
  title: { absolute: HOME_TITLE },
  description: HOME_DESCRIPTION,
  alternates: {
    canonical: HOME_CANONICAL_URL,
  },
  openGraph: {
    type: 'website',
    siteName: HOME_TITLE,
    locale: 'en_US',
    url: HOME_CANONICAL_URL,
    title: HOME_TITLE,
    description: HOME_DESCRIPTION,
    images: [
      {
        url: `${BASE_PATH}/og/image.png`,
        width: 1200,
        height: 630,
        alt: HOME_TITLE,
      },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    title: HOME_TITLE,
    description: HOME_DESCRIPTION,
    images: [
      {
        url: `${BASE_PATH}/og/image.png`,
        alt: HOME_TITLE,
      },
    ],
  },
};

const homeJsonLd = {
  '@context': 'https://schema.org',
  '@graph': [
    {
      '@type': 'SoftwareApplication',
      name: HOME_TITLE,
      description: HOME_DESCRIPTION,
      url: HOME_CANONICAL_URL,
      downloadUrl: toCanonicalPageUrl('/download'),
      codeRepository: REPO_URL,
      operatingSystem: 'Linux',
      applicationCategory: 'UtilitiesApplication',
      license: 'https://www.gnu.org/licenses/gpl-3.0.html',
      isAccessibleForFree: true,
      offers: {
        '@type': 'Offer',
        price: '0',
        priceCurrency: 'USD',
      },
      author: {
        '@type': 'Person',
        name: 'Azorimor',
        url: 'https://github.com/Azorimor',
      },
    },
    {
      '@type': 'WebSite',
      name: HOME_TITLE,
      url: HOME_CANONICAL_URL,
      description: HOME_DESCRIPTION,
    },
  ],
};

const features = [
  {
    icon: Usb,
    title: 'Wired USB Controller Check',
    description:
      'Plug in your controller with a USB cable to view its current firmware version, model, shell color, and battery status right away.',
  },
  {
    icon: Cpu,
    title: 'Automatic Hardware Detection',
    description:
      "Automatically detects your controller's hardware, picks the matching official firmware update, and blocks unsupported early launch models.",
  },
  {
    icon: ShieldAlert,
    title: 'Guided Safety & Recovery',
    description:
      'Watches for accidental cable unplugs before updating starts, confirms you are ready before making changes, and explains any errors in plain English.',
  },
];

const screenshots = [
  {
    step: 'Step 1 — Initial Scan',
    title: 'Waiting for a Wired USB Connection',
    caption:
      'Prompts you to connect your DualSense controller directly over a USB data cable and warns that updates cannot run over Bluetooth.',
    image: screenshotInitial,
    alt: 'Unofficial Linux DualSense Firmware Updater initial screen waiting for a wired USB controller connection',
  },
  {
    step: 'Step 1 — Controller Detected',
    title: 'Controller Model & Firmware Check',
    caption:
      'Displays the detected controller model, shell color, current firmware version, and firmware build details once plugged in over USB.',
    image: screenshotController,
    alt: 'Unofficial Linux DualSense Firmware Updater displaying a detected Midnight Black DualSense controller over USB',
  },
  {
    step: 'Step 2 — Firmware Source',
    title: 'Official Sony Update or Local Firmware File',
    caption:
      "Checks Sony's server for the official firmware update matching your controller, or lets you pick a firmware file saved on your computer.",
    image: screenshotFirmware,
    alt: 'Unofficial Linux DualSense Firmware Updater firmware selection step with official Sony download and local file options',
  },
  {
    step: 'Step 3 — Review & Consent',
    title: 'Version Comparison & Safety Confirmation',
    caption:
      'Compares your current and new firmware versions, warns if you already have the latest version or an untested DualSense Edge controller, and asks you to confirm before updating.',
    image: screenshotReview,
    alt: 'Unofficial Linux DualSense Firmware Updater review and confirmation screen showing version comparison and safety notices',
  },
];

export default function HomePage() {
  return (
    <main className="mx-auto flex w-full max-w-6xl flex-1 flex-col gap-10 px-6 py-12 md:py-16">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{
          __html: JSON.stringify(homeJsonLd).replace(/</g, '\\u003c'),
        }}
      />

      <section className="flex flex-col items-center gap-6 text-center">
        <h1 className="max-w-3xl text-4xl font-bold tracking-tight text-fd-foreground sm:text-5xl">
          Unofficial Linux DualSense Firmware Updater
        </h1>

        <p className="max-w-2xl text-base text-fd-muted-foreground sm:text-lg">
          An unofficial, community-maintained Linux app for checking and
          updating PlayStation DualSense and DualSense Edge controller firmware
          over a wired USB cable. This project has no affiliation, endorsement,
          or relation to Sony Interactive Entertainment or PlayStation.
        </p>

        <div className="flex flex-wrap items-center justify-center gap-3 pt-2">
          <Link
            href="/download"
            className="inline-flex items-center gap-2 rounded-lg bg-fd-primary px-5 py-2.5 text-sm font-medium text-fd-primary-foreground transition-opacity hover:opacity-90"
          >
            <Download className="size-4" aria-hidden="true" />
            Download
          </Link>
          <Link
            href="/docs"
            className="inline-flex items-center gap-2 rounded-lg border border-fd-border bg-fd-card px-5 py-2.5 text-sm font-medium text-fd-card-foreground transition-colors hover:bg-fd-accent"
          >
            <BookOpen className="size-4" aria-hidden="true" />
            Read Documentation
          </Link>
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
                <Icon className="size-5" aria-hidden="true" />
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
            Simple 4-Step Update Workflow
          </h2>
          <p className="mx-auto max-w-2xl text-sm text-fd-muted-foreground sm:text-base">
            Step-by-step screenshots of the Unofficial Linux DualSense Firmware
            Updater from connecting over USB through choosing an update and
            confirming before changes begin.
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
          <AlertTriangle
            className="mt-0.5 size-5 shrink-0 text-amber-500"
            aria-hidden="true"
          />
          <div className="space-y-2">
            <p className="font-semibold">
              Unofficial Community Project &mdash; No Sony Affiliation
            </p>
            <p className="leading-relaxed text-fd-muted-foreground">
              This app is an unofficial, community-maintained tool with no
              affiliation, endorsement, or relation to Sony Interactive
              Entertainment or PlayStation. Updating firmware can permanently
              damage your controller, and once an update starts it cannot be
              undone. Only use this tool if you have no access to Sony&apos;s
              official Windows updater or a PlayStation 5 console. Standard
              DualSense controllers are tested; DualSense Edge remains untested
              and is clearly flagged before updating.
            </p>
          </div>
        </div>
      </section>
    </main>
  );
}
