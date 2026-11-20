// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import type { Metadata } from 'next';
import Link from 'next/link';
import { HomeLayout } from 'fumadocs-ui/layouts/home';
import {
  AlertTriangle,
  BookOpen,
  CheckCircle2,
  Cpu,
  Download,
  Usb,
  XCircle,
} from 'lucide-react';
import { ControllerChecker } from '@/components/ControllerChecker';
import { SiteFooter } from '@/components/SiteFooter';
import { BASE_PATH, baseOptions } from '@/lib/layout.shared';
import { toCanonicalPageUrl } from '@/lib/llms-urls';

export const dynamic = 'force-static';

const PAGE_TITLE = 'Supported Hardware';
const PAGE_DESCRIPTION =
  'Check if your PlayStation DualSense or DualSense Edge controller is supported for Linux firmware updates, including an instant browser USB checker.';
const CANONICAL_URL = toCanonicalPageUrl('/supported-hardware');
const HOME_URL = toCanonicalPageUrl('/');
const OG_IMAGE_URL = `${BASE_PATH}/og/supported-hardware/image.png`;

export const metadata: Metadata = {
  title: PAGE_TITLE,
  description: PAGE_DESCRIPTION,
  alternates: {
    canonical: CANONICAL_URL,
  },
  openGraph: {
    type: 'website',
    siteName: 'Unofficial Linux DualSense Firmware Updater',
    locale: 'en_US',
    url: CANONICAL_URL,
    title: PAGE_TITLE,
    description: PAGE_DESCRIPTION,
    images: [
      {
        url: OG_IMAGE_URL,
        width: 1200,
        height: 630,
        alt: 'Supported Hardware — Unofficial Linux DualSense Firmware Updater',
      },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    title: PAGE_TITLE,
    description: PAGE_DESCRIPTION,
    images: [
      {
        url: OG_IMAGE_URL,
        alt: 'Supported Hardware — Unofficial Linux DualSense Firmware Updater',
      },
    ],
  },
};

const hardwareMatrixRows = [
  {
    model: 'DualSense (BDM-010)',
    usbIds: '0x054c:0x0ce6',
    hardwareByte: '0x03',
    targetId: '—',
    status: 'Unsupported (not in Sony’s PC catalog; update via PS5)',
  },
  {
    model: 'DualSense (BDM-020)',
    usbIds: '0x054c:0x0ce6',
    hardwareByte: '0x04',
    targetId: '0004',
    status: 'Supported & tested',
  },
  {
    model: 'DualSense (BDM-030)',
    usbIds: '0x054c:0x0ce6',
    hardwareByte: '0x05',
    targetId: '0004',
    status: 'Supported & tested',
  },
  {
    model: 'DualSense (BDM-040)',
    usbIds: '0x054c:0x0ce6',
    hardwareByte: '0x06',
    targetId: '0004',
    status: 'Supported & tested',
  },
  {
    model: 'DualSense (BDM-050)',
    usbIds: '0x054c:0x0ce6',
    hardwareByte: '0x07 / 0x08',
    targetId: '000B',
    status: 'Supported & tested',
  },
  {
    model: 'DualSense (BDM-060R)',
    usbIds: '0x054c:0x0ce6',
    hardwareByte: '0x09',
    targetId: '000E',
    status: 'Supported & tested',
  },
  {
    model: 'DualSense (BDM-060M)',
    usbIds: '0x054c:0x0ce6',
    hardwareByte: '0x11',
    targetId: '000E',
    status: 'Supported & tested',
  },
  {
    model: 'DualSense (BDM-060X)',
    usbIds: '0x054c:0x0ce6',
    hardwareByte: '0x13',
    targetId: '000E',
    status: 'Supported & tested',
  },
  {
    model: 'DualSense Edge',
    usbIds: '0x054c:0x0df2',
    hardwareByte: 'PID 0x0df2',
    targetId: '0044',
    status: 'Supported in catalog (Untested)',
  },
];

const structuredData = {
  '@context': 'https://schema.org',
  '@graph': [
    {
      '@type': 'WebPage',
      '@id': CANONICAL_URL,
      url: CANONICAL_URL,
      name: 'Supported Controllers & Hardware',
      description: PAGE_DESCRIPTION,
      isPartOf: {
        '@type': 'WebSite',
        url: HOME_URL,
        name: 'Unofficial Linux DualSense Firmware Updater',
      },
      breadcrumb: {
        '@id': `${CANONICAL_URL}#breadcrumb`,
      },
    },
    {
      '@type': 'BreadcrumbList',
      '@id': `${CANONICAL_URL}#breadcrumb`,
      itemListElement: [
        {
          '@type': 'ListItem',
          position: 1,
          name: 'Home',
          item: HOME_URL,
        },
        {
          '@type': 'ListItem',
          position: 2,
          name: 'Supported Hardware',
          item: CANONICAL_URL,
        },
      ],
    },
  ],
};

export default function SupportedHardwarePage() {
  return (
    <HomeLayout {...baseOptions()}>
      <main className="flex-1">
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify(structuredData).replace(/</g, '\\u003c'),
          }}
        />

        <div className="mx-auto flex w-full max-w-5xl flex-col gap-12 px-6 py-12 md:py-16">
          <header className="space-y-3">
            <h1 className="text-3xl font-bold tracking-tight text-fd-foreground sm:text-4xl">
              Supported Controller Hardware
            </h1>
            <p className="max-w-3xl text-base leading-relaxed text-fd-muted-foreground">
              Most Sony PlayStation DualSense controllers released after the
              2020 launch work with this unofficial Linux firmware updater. Use the instant
              USB checker below to see right away whether your controller is
              supported.
            </p>
          </header>

          <section className="space-y-4">
            <h2 className="text-2xl font-bold tracking-tight text-fd-foreground">
              Check Your Controller in the Browser
            </h2>
            <ControllerChecker />
          </section>

          <section className="space-y-5">
            <h2 className="text-2xl font-bold tracking-tight text-fd-foreground">
              Supported Models at a Glance
            </h2>
            <div className="grid gap-6 md:grid-cols-3">
              <div className="flex flex-col gap-3 rounded-xl border border-fd-border bg-fd-card p-6">
                <div className="flex items-center justify-between gap-2">
                  <span className="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/40 bg-emerald-500/10 px-2.5 py-0.5 text-xs font-semibold text-emerald-600 dark:text-emerald-400">
                    <CheckCircle2
                      className="size-3.5 shrink-0"
                      aria-hidden="true"
                    />
                    <span>Supported, Tested</span>
                  </span>
                </div>
                <h3 className="text-lg font-semibold text-fd-card-foreground">
                  Standard DualSense
                </h3>
                <p className="text-sm leading-relaxed text-fd-muted-foreground">
                  Standard DualSense wireless controllers built after the 2020
                  launch window are fully supported and tested on Linux for
                  official Sony firmware updates.
                </p>
              </div>

              <div className="flex flex-col gap-3 rounded-xl border border-fd-border bg-fd-card p-6">
                <div className="flex items-center justify-between gap-2">
                  <span className="inline-flex items-center gap-1.5 rounded-full border border-amber-500/40 bg-amber-500/10 px-2.5 py-0.5 text-xs font-semibold text-amber-600 dark:text-amber-400">
                    <AlertTriangle
                      className="size-3.5 shrink-0"
                      aria-hidden="true"
                    />
                    <span>Supported, Untested</span>
                  </span>
                </div>
                <h3 className="text-lg font-semibold text-fd-card-foreground">
                  DualSense Edge
                </h3>
                <p className="text-sm leading-relaxed text-fd-muted-foreground">
                  DualSense Edge firmware exists in the official Sony catalog,
                  but flashing has not yet been tested on real DualSense Edge
                  hardware. The app flags this clearly before flashing.
                </p>
              </div>

              <div className="flex flex-col gap-3 rounded-xl border border-fd-border bg-fd-card p-6">
                <div className="flex items-center justify-between gap-2">
                  <span className="inline-flex items-center gap-1.5 rounded-full border border-rose-500/40 bg-rose-500/10 px-2.5 py-0.5 text-xs font-semibold text-rose-600 dark:text-rose-400">
                    <XCircle
                      className="size-3.5 shrink-0"
                      aria-hidden="true"
                    />
                    <span>Not Supported</span>
                  </span>
                </div>
                <h3 className="text-lg font-semibold text-fd-card-foreground">
                  Original 2020 Launch DualSense
                </h3>
                <p className="text-sm leading-relaxed text-fd-muted-foreground">
                  Earliest 2020 launch DualSense controllers (BDM-010) are not
                  part of the official Sony PC update catalog. The app detects them
                  and blocks updates.
                </p>
              </div>
            </div>
          </section>

          <section className="space-y-5">
            <h2 className="text-2xl font-bold tracking-tight text-fd-foreground">
              What You Need
            </h2>
            <div className="grid gap-6 md:grid-cols-2">
              <div className="flex flex-col gap-3 rounded-xl border border-fd-border bg-fd-card p-6">
                <div className="inline-flex size-10 items-center justify-center rounded-lg border border-fd-border bg-fd-secondary text-fd-foreground">
                  <Usb className="size-5" aria-hidden="true" />
                </div>
                <h3 className="text-lg font-semibold text-fd-card-foreground">
                  Wired USB-C Data Cable
                </h3>
                <p className="text-sm leading-relaxed text-fd-muted-foreground">
                  Firmware updates can only run over a direct, data-capable
                  USB-C cable. Bluetooth connections and power-only charging
                  cables cannot flash controller firmware.
                </p>
              </div>

              <div className="flex flex-col gap-3 rounded-xl border border-fd-border bg-fd-card p-6">
                <div className="inline-flex size-10 items-center justify-center rounded-lg border border-fd-border bg-fd-secondary text-fd-foreground">
                  <Cpu className="size-5" aria-hidden="true" />
                </div>
                <h3 className="text-lg font-semibold text-fd-card-foreground">
                  Automatic Detection
                </h3>
                <p className="text-sm leading-relaxed text-fd-muted-foreground">
                  You do not need to open your controller or look up serial
                  numbers. The Linux desktop app automatically identifies your
                  board revision, fetches the matching Sony image, and prevents
                  mismatched flashes.
                </p>
              </div>
            </div>
          </section>

          <section className="space-y-4">
            <h2 className="text-2xl font-bold tracking-tight text-fd-foreground">
              Technical Reference
            </h2>
            <details className="rounded-xl border border-fd-border bg-fd-card p-6">
              <summary className="cursor-pointer font-medium text-fd-foreground">
                Show board revisions, USB IDs, and firmware targets
              </summary>
              <div className="mt-5 space-y-5 text-sm text-fd-muted-foreground">
                <p className="leading-relaxed">
                  The updater reads HID Feature Report <code>0x20</code> (
                  <code>(hw_info &gt;&gt; 8) &amp; 0xff</code> and{' '}
                  <code>fw_series</code>) together with the USB Product ID to
                  map your controller to its official Sony firmware target. When
                  permitted, Feature Reports <code>0x80</code> and{' '}
                  <code>0x81</code> are used only to identify the controller
                  shell colorway.
                </p>

                <div
                  tabIndex={0}
                  role="region"
                  aria-labelledby="hardware-matrix-caption"
                  className="responsive-table-wrapper"
                >
                  <table>
                    <caption id="hardware-matrix-caption">
                      DualSense hardware revisions, USB vendor/product IDs,
                      Feature Report 0x20 bytes, and Sony firmware targets
                    </caption>
                    <thead>
                      <tr>
                        <th scope="col">Board Revision</th>
                        <th scope="col">USB VID:PID</th>
                        <th scope="col">Hardware Byte</th>
                        <th scope="col">Sony Target</th>
                        <th scope="col">Status</th>
                      </tr>
                    </thead>
                    <tbody>
                      {hardwareMatrixRows.map((row) => (
                        <tr key={row.model}>
                          <th scope="row" data-label="Board Revision">
                            {row.model}
                          </th>
                          <td data-label="USB VID:PID">
                            <code>{row.usbIds}</code>
                          </td>
                          <td data-label="Hardware Byte">
                            <code>{row.hardwareByte}</code>
                          </td>
                          <td data-label="Sony Target">
                            <code>{row.targetId}</code>
                          </td>
                          <td data-label="Status">{row.status}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>

                <div className="space-y-2 rounded-lg border border-fd-border bg-fd-background p-4 text-xs leading-relaxed">
                  <p className="font-semibold text-fd-foreground">
                    Official Sony Firmware Endpoints
                  </p>
                  <p>
                    Version metadata:{' '}
                    <code className="break-all">
                      https://fwupdater.dl.playstation.net/fwupdater/info.json
                    </code>
                  </p>
                  <p>
                    Firmware binaries:{' '}
                    <code className="break-all">
                      https://fwupdater.dl.playstation.net/fwupdater/fwupdate&lt;target&gt;/&lt;version&gt;/FWUPDATE&lt;target&gt;.bin
                    </code>
                  </p>
                </div>
              </div>
            </details>
          </section>

          <section className="flex flex-wrap items-center gap-4 border-t border-fd-border pt-6">
            <Link
              href="/download"
              className="inline-flex items-center gap-2 rounded-lg bg-fd-primary px-5 py-2.5 text-sm font-medium text-fd-primary-foreground transition-opacity hover:opacity-90"
            >
              <Download className="size-4 shrink-0" aria-hidden="true" />
              <span>Download the Linux App</span>
            </Link>
            <Link
              href="/docs/safety-and-troubleshooting"
              className="inline-flex items-center gap-2 rounded-lg border border-fd-border bg-fd-card px-5 py-2.5 text-sm font-medium text-fd-card-foreground transition-colors hover:bg-fd-accent"
            >
              <BookOpen className="size-4 shrink-0" aria-hidden="true" />
              <span>Safety and Troubleshooting Guide</span>
            </Link>
          </section>
        </div>
      </main>
      <SiteFooter />
    </HomeLayout>
  );
}
