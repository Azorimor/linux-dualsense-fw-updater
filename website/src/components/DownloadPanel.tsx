// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

'use client';

import { useState } from 'react';
import Link from 'next/link';
import {
  AlertTriangle,
  Download,
  ExternalLink,
  FileCheck2,
  Package,
  Terminal,
} from 'lucide-react';
import {
  formatPublishedDate,
  type GitHubRelease,
  type ReleaseAsset,
  type ReleaseAssetKind,
} from '@/lib/github-release';
import { REPO_URL } from '@/lib/layout.shared';

interface DownloadPanelProps {
  release: GitHubRelease | null;
}

const ASSET_LABELS: Record<ReleaseAssetKind, string> = {
  appimage: 'Linux AppImage (x86_64)',
  tarball: 'Portable Archive (.tar.gz)',
  checksums: 'SHA-256 Checksums',
};

function AssetDownloadAction({
  asset,
  acknowledged,
}: {
  asset: ReleaseAsset;
  acknowledged: boolean;
}) {
  const Icon =
    asset.kind === 'checksums'
      ? FileCheck2
      : asset.kind === 'tarball'
        ? Package
        : Download;

  if (!acknowledged) {
    return (
      <button
        type="button"
        disabled
        aria-disabled="true"
        className="inline-flex w-full cursor-not-allowed items-center justify-between gap-3 rounded-lg border border-fd-border bg-fd-muted px-4 py-3 text-left text-sm text-fd-muted-foreground opacity-55 sm:w-auto sm:min-w-64"
      >
        <span className="inline-flex items-center gap-2 font-medium">
          <Icon className="size-4 shrink-0" />
          <span>{ASSET_LABELS[asset.kind]}</span>
        </span>
        <span className="text-xs">{asset.formattedSize}</span>
      </button>
    );
  }

  return (
    <a
      href={asset.downloadUrl}
      rel="noreferrer"
      className="inline-flex w-full items-center justify-between gap-3 rounded-lg border border-fd-border bg-fd-primary px-4 py-3 text-left text-sm text-fd-primary-foreground transition-opacity hover:opacity-90 sm:w-auto sm:min-w-64"
    >
      <span className="inline-flex items-center gap-2 font-medium">
        <Icon className="size-4 shrink-0" />
        <span>{ASSET_LABELS[asset.kind]}</span>
      </span>
      <span className="text-xs opacity-80">{asset.formattedSize}</span>
    </a>
  );
}

export function DownloadPanel({ release }: DownloadPanelProps) {
  const [acknowledged, setAcknowledged] = useState(false);

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-8 px-6 py-12 md:py-16">
      <header className="space-y-3">
        <span className="inline-flex items-center rounded-full border border-fd-border bg-fd-secondary px-3 py-1 text-xs font-medium text-fd-secondary-foreground">
          Unofficial Community Release Downloads
        </span>
        <h1 className="text-3xl font-bold tracking-tight text-fd-foreground sm:text-4xl">
          Download Unofficial Linux DualSense Firmware Updater
        </h1>
        <p className="text-base text-fd-muted-foreground">
          An unofficial, community-maintained desktop utility with no
          affiliation, endorsement, or relation to Sony Interactive
          Entertainment or PlayStation.
        </p>
      </header>

      <section
        aria-label="Mandatory safety warning"
        className="rounded-xl border border-amber-500/40 bg-amber-500/10 p-6 text-sm text-fd-foreground"
      >
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-0.5 size-5 shrink-0 text-amber-500" />
          <div className="space-y-3">
            <h2 className="text-base font-semibold">
              Read Before Downloading &amp; Flashing
            </h2>
            <ul className="list-disc space-y-2 pl-5 leading-relaxed text-fd-muted-foreground">
              <li>
                <strong className="text-fd-foreground">
                  Unofficial community-maintained tool:
                </strong>{' '}
                This software is developed independently and has no relation to
                Sony Interactive Entertainment or PlayStation.
              </li>
              <li>
                <strong className="text-fd-foreground">
                  Risk of permanent controller damage:
                </strong>{' '}
                Flashing firmware can permanently damage or brick your
                controller. Upgrades are auto-committed by the controller even
                if verify and finalize steps do not run.
              </li>
              <li>
                <strong className="text-fd-foreground">
                  Use official Sony tools whenever possible:
                </strong>{' '}
                Only use this software if you have{' '}
                <strong className="text-fd-foreground">
                  absolutely no access
                </strong>{' '}
                to Sony&apos;s official Windows firmware updater or a
                PlayStation 5 console (neither your own nor a friend&apos;s).
              </li>
            </ul>
          </div>
        </div>
      </section>

      <section className="rounded-xl border border-fd-border bg-fd-card p-6">
        <label className="flex cursor-pointer items-start gap-3 text-sm leading-relaxed text-fd-foreground">
          <input
            type="checkbox"
            checked={acknowledged}
            onChange={(event) => setAcknowledged(event.target.checked)}
            className="mt-1 size-4 shrink-0 rounded border border-fd-border accent-fd-primary"
          />
          <span>
            I understand that this is an unofficial community tool with no
            relation to Sony, that flashing firmware can break my controller,
            and that I have no access to an official Windows PC updater or a
            PlayStation 5 console.
          </span>
        </label>
      </section>

      <section className="space-y-5 rounded-xl border border-fd-border bg-fd-card p-6">
        {release ? (
          <>
            <div className="flex flex-wrap items-baseline justify-between gap-2 border-b border-fd-border pb-4">
              <div>
                <h2 className="text-xl font-semibold text-fd-card-foreground">
                  Latest Release: {release.name}
                </h2>
                <p className="mt-1 text-xs text-fd-muted-foreground">
                  Tag <code>{release.tagName}</code> &bull; Published{' '}
                  {formatPublishedDate(release.publishedAt)}
                </p>
              </div>
              <a
                href={release.htmlUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1.5 text-sm font-medium text-fd-muted-foreground hover:text-fd-foreground"
              >
                Release Notes
                <ExternalLink className="size-3.5" />
              </a>
            </div>

            {release.assets.length > 0 ? (
              <div className="space-y-3">
                <p className="text-sm text-fd-muted-foreground">
                  {acknowledged
                    ? 'Acknowledgment confirmed. Select an artifact below to download:'
                    : 'Check the acknowledgment box above to unlock the download buttons.'}
                </p>
                <div className="flex flex-wrap gap-3">
                  {release.assets.map((asset) => (
                    <AssetDownloadAction
                      key={asset.name}
                      asset={asset}
                      acknowledged={acknowledged}
                    />
                  ))}
                </div>
                <ul className="space-y-1 pt-2 text-xs text-fd-muted-foreground">
                  {release.assets.map((asset) => (
                    <li key={asset.name}>
                      <code>{asset.name}</code> ({asset.formattedSize})
                    </li>
                  ))}
                </ul>
              </div>
            ) : (
              <div className="space-y-3 text-sm text-fd-muted-foreground">
                <p>
                  Release <code>{release.tagName}</code> exists, but no
                  pre-built <code>.AppImage</code>, <code>.tar.gz</code>, or{' '}
                  <code>SHA256SUMS.txt</code> assets were found on it.
                </p>
                <button
                  type="button"
                  disabled
                  aria-disabled="true"
                  className="inline-flex cursor-not-allowed items-center gap-2 rounded-lg border border-fd-border bg-fd-muted px-4 py-2.5 text-sm font-medium text-fd-muted-foreground opacity-55"
                >
                  <Download className="size-4" />
                  No Release Binaries Attached
                </button>
              </div>
            )}
          </>
        ) : (
          <div className="space-y-4">
            <div className="border-b border-fd-border pb-4">
              <h2 className="text-xl font-semibold text-fd-card-foreground">
                No release published yet
              </h2>
              <p className="mt-1 text-sm text-fd-muted-foreground">
                No tagged release is currently available for download. Once a
                version tag is published, this page will list the latest{' '}
                <code>.AppImage</code>, <code>.tar.gz</code>, and{' '}
                <code>SHA256SUMS.txt</code> artifacts automatically at build
                time.
              </p>
            </div>

            <div className="flex flex-wrap items-center gap-3">
              <button
                type="button"
                disabled
                aria-disabled="true"
                className="inline-flex cursor-not-allowed items-center gap-2 rounded-lg border border-fd-border bg-fd-muted px-4 py-2.5 text-sm font-medium text-fd-muted-foreground opacity-55"
              >
                <Download className="size-4" />
                {acknowledged
                  ? 'No Release Published Yet'
                  : 'Acknowledge Warning to Download (No Release Yet)'}
              </button>

              <Link
                href="/docs/getting-started"
                className="inline-flex items-center gap-2 rounded-lg border border-fd-border bg-fd-secondary px-4 py-2.5 text-sm font-medium text-fd-secondary-foreground transition-colors hover:bg-fd-accent"
              >
                <Terminal className="size-4" />
                Build from Source
              </Link>

              <a
                href={`${REPO_URL}/releases`}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1.5 text-sm font-medium text-fd-muted-foreground hover:text-fd-foreground"
              >
                GitHub Releases
                <ExternalLink className="size-3.5" />
              </a>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
