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
  partitionReleaseAssets,
  type GitHubRelease,
  type ReleaseAsset,
} from '@/lib/github-release';
import { REPO_URL } from '@/lib/layout.shared';

interface DownloadPanelProps {
  release: GitHubRelease | null;
}

interface BuildVariantMeta {
  title: string;
  badge: string;
  isRecommended: boolean;
  subtitle: string;
  buttonLabel: string;
}

function describeBuildVariant(asset: ReleaseAsset): BuildVariantMeta {
  if (asset.kind === 'tarball') {
    return {
      title: 'Portable Archive (.tar.gz)',
      badge: 'Recommended',
      isRecommended: true,
      subtitle:
        'Extract and run right away on x86_64 Linux, or optionally add an app menu entry and icon with ./install-desktop.sh.',
      buttonLabel: 'Download .tar.gz',
    };
  }
  return {
    title: 'Linux AppImage (x86_64)',
    badge: 'Alternative',
    isRecommended: false,
    subtitle:
      'All-in-one executable file bundling its own graphics runtime and desktop icon (mark executable with chmod +x).',
    buttonLabel: 'Download AppImage',
  };
}

function BuildDownloadCard({
  asset,
  acknowledged,
}: {
  asset: ReleaseAsset;
  acknowledged: boolean;
}) {
  const meta = describeBuildVariant(asset);
  const Icon = asset.kind === 'tarball' ? Package : Download;

  return (
    <div
      className={
        meta.isRecommended
          ? 'flex flex-col justify-between gap-4 rounded-xl border border-fd-primary/50 bg-fd-background p-5'
          : 'flex flex-col justify-between gap-4 rounded-xl border border-fd-border bg-fd-background p-5'
      }
    >
      <div className="space-y-2">
        <div className="flex items-start justify-between gap-2">
          <div className="space-y-1.5">
            <div className="flex items-center gap-2 font-semibold text-fd-foreground">
              <Icon className="size-4 shrink-0" aria-hidden="true" />
              <h3>{meta.title}</h3>
            </div>
            <span
              className={
                meta.isRecommended
                  ? 'inline-flex items-center rounded-md border border-fd-primary/30 bg-fd-primary/15 px-2 py-0.5 text-xs font-semibold text-fd-foreground'
                  : 'inline-flex items-center rounded-md border border-fd-border bg-fd-secondary px-2 py-0.5 text-xs font-medium text-fd-muted-foreground'
              }
            >
              {meta.badge}
            </span>
          </div>
          <span className="shrink-0 rounded-md border border-fd-border bg-fd-secondary px-2 py-0.5 text-xs font-medium text-fd-secondary-foreground">
            {asset.formattedSize}
          </span>
        </div>

        <p className="text-xs leading-relaxed text-fd-muted-foreground">
          {meta.subtitle}
        </p>

        <div className="space-y-1 pt-1 text-xs text-fd-muted-foreground">
          <div className="truncate font-mono" title={asset.name}>
            {asset.name}
          </div>
          {asset.sha256 ? (
            <div className="break-all font-mono text-[11px] opacity-85">
              sha256:{asset.sha256}
            </div>
          ) : null}
        </div>
      </div>

      {acknowledged ? (
        <a
          href={asset.downloadUrl}
          rel="noreferrer"
          className={
            meta.isRecommended
              ? 'inline-flex w-full items-center justify-center gap-2 rounded-lg border border-fd-border bg-fd-primary px-4 py-2.5 text-sm font-medium text-fd-primary-foreground transition-opacity hover:opacity-90'
              : 'inline-flex w-full items-center justify-center gap-2 rounded-lg border border-fd-border bg-fd-secondary px-4 py-2.5 text-sm font-medium text-fd-secondary-foreground transition-colors hover:bg-fd-accent'
          }
        >
          <Icon className="size-4 shrink-0" aria-hidden="true" />
          <span>{meta.buttonLabel}</span>
        </a>
      ) : (
        <button
          type="button"
          disabled
          aria-disabled="true"
          className="inline-flex w-full cursor-not-allowed items-center justify-center gap-2 rounded-lg border border-fd-border bg-fd-muted px-4 py-2.5 text-sm font-medium text-fd-muted-foreground opacity-55"
        >
          <Icon className="size-4 shrink-0" aria-hidden="true" />
          <span>{meta.buttonLabel}</span>
        </button>
      )}
    </div>
  );
}

export function DownloadPanel({ release }: DownloadPanelProps) {
  const [acknowledged, setAcknowledged] = useState(false);

  const partitioned = release ? partitionReleaseAssets(release.assets) : null;
  const buildAssets = partitioned
    ? [
        ...(partitioned.tarball ? [partitioned.tarball] : []),
        ...(partitioned.appimage ? [partitioned.appimage] : []),
        ...partitioned.otherBuilds,
      ]
    : [];
  const checksumsAsset = partitioned?.checksums ?? null;

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-8 px-6 py-12 md:py-16">
      <header className="space-y-3">
        <h1 className="text-3xl font-bold tracking-tight text-fd-foreground sm:text-4xl">
          Download Unofficial Linux DualSense Firmware Updater
        </h1>
        <p className="text-base text-fd-muted-foreground">
          An unofficial, community-maintained Linux app with no affiliation,
          endorsement, or relation to Sony Interactive Entertainment or
          PlayStation.
        </p>
      </header>

      <section
        aria-label="Mandatory safety warning"
        className="rounded-xl border border-amber-500/40 bg-amber-500/10 p-6 text-sm text-fd-foreground"
      >
        <div className="flex items-start gap-3">
          <AlertTriangle
            className="mt-0.5 size-5 shrink-0 text-amber-500"
            aria-hidden="true"
          />
          <div className="space-y-3">
            <h2 className="text-base font-semibold">
              Read Before Downloading and Updating
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
                Updating firmware can permanently damage your controller. Once
                an update starts, the controller saves it permanently and it
                cannot be undone.
              </li>
              <li>
                <strong className="text-fd-foreground">
                  Use official Sony tools whenever possible:
                </strong>{' '}
                Only use this software if you have{' '}
                <strong className="text-fd-foreground">
                  absolutely no access
                </strong>{' '}
                to Sony's official Windows firmware updater or a
                PlayStation 5 console (neither your own nor a friend's).
                This project is not responsible for bricked controllers.
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
            relation to Sony, that updating firmware can permanently damage my
            controller, and that I have no access to an official Windows PC
            updater or a PlayStation 5 console to update the firmware.
          </span>
        </label>
      </section>

      <section className="space-y-6 rounded-xl border border-fd-border bg-fd-card p-6">
        {release ? (
          <>
            <div className="flex flex-wrap items-baseline justify-between gap-2 border-b border-fd-border pb-4">
              <div>
                <div className="flex flex-wrap items-center gap-2.5">
                  <h2 className="text-xl font-semibold text-fd-card-foreground">
                    Latest Release: {release.name}
                  </h2>
                  <span className="rounded-md border border-fd-border bg-fd-secondary px-2 py-0.5 text-xs font-medium text-fd-secondary-foreground">
                    {release.tagName}
                  </span>
                </div>
                <p className="mt-1 text-xs text-fd-muted-foreground">
                  Published {formatPublishedDate(release.publishedAt)}
                </p>
              </div>
              <a
                href={release.htmlUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1.5 text-sm font-medium text-fd-muted-foreground hover:text-fd-foreground"
              >
                GitHub Release Page
                <ExternalLink className="size-3.5" aria-hidden="true" />
              </a>
            </div>

            {buildAssets.length > 0 ? (
              <div className="space-y-5">
                <p className="text-sm text-fd-muted-foreground">
                  {acknowledged
                    ? 'Acknowledgment confirmed. Choose the recommended portable archive or the alternative AppImage below:'
                    : 'Check the acknowledgment box above to unlock both download buttons.'}
                </p>
                <p className="text-sm">
                  Downloads are served directly from GitHub.
                </p>

                <div className="grid gap-4 sm:grid-cols-2">
                  {buildAssets.map((asset) => (
                    <BuildDownloadCard
                      key={asset.name}
                      asset={asset}
                      acknowledged={acknowledged}
                    />
                  ))}
                </div>

                {checksumsAsset ? (
                  <div className="flex flex-col justify-between gap-4 rounded-xl border border-fd-border bg-fd-background p-4 sm:flex-row sm:items-center">
                    <div className="space-y-1">
                      <div className="flex items-center gap-2 text-sm font-medium text-fd-foreground">
                        <FileCheck2
                          className="size-4 shrink-0"
                          aria-hidden="true"
                        />
                        <span>SHA-256 Checksums ({checksumsAsset.name})</span>
                        <span className="text-xs font-normal text-fd-muted-foreground">
                          ({checksumsAsset.formattedSize})
                        </span>
                      </div>
                      <p className="text-xs text-fd-muted-foreground">
                        Optional: check your downloaded file integrity with{' '}
                        <code className="rounded bg-fd-muted px-1.5 py-0.5 font-mono text-[11px] text-fd-foreground">
                          sha256sum -c {checksumsAsset.name}
                        </code>
                      </p>
                      {checksumsAsset.sha256 ? (
                        <p className="break-all font-mono text-[11px] text-fd-muted-foreground">
                          sha256:{checksumsAsset.sha256}
                        </p>
                      ) : null}
                    </div>

                    <a
                      href={checksumsAsset.downloadUrl}
                      rel="noreferrer"
                      className="inline-flex shrink-0 items-center justify-center gap-2 rounded-lg border border-fd-border bg-fd-secondary px-4 py-2 text-sm font-medium text-fd-secondary-foreground transition-colors hover:bg-fd-accent"
                    >
                      <FileCheck2
                        className="size-4 shrink-0"
                        aria-hidden="true"
                      />
                      <span>Download {checksumsAsset.name}</span>
                      <ExternalLink
                        className="size-3.5 shrink-0 opacity-75"
                        aria-hidden="true"
                      />
                    </a>
                  </div>
                ) : null}
              </div>
            ) : (
              <div className="space-y-3 text-sm text-fd-muted-foreground">
                <p>
                  Release <code>{release.tagName}</code> exists, but no
                  pre-built Portable Archive (<code>.tar.gz</code>,{' '}
                  Recommended), <code>.AppImage</code>, or{' '}
                  <code>SHA256SUMS.txt</code> files were found on it.
                </p>
                <button
                  type="button"
                  disabled
                  aria-disabled="true"
                  className="inline-flex cursor-not-allowed items-center gap-2 rounded-lg border border-fd-border bg-fd-muted px-4 py-2.5 text-sm font-medium text-fd-muted-foreground opacity-55"
                >
                  <Download className="size-4" aria-hidden="true" />
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
                version tag is published, this page will list the Portable
                Archive (<code>.tar.gz</code>, Recommended),{' '}
                <code>.AppImage</code>, and <code>SHA256SUMS.txt</code> files
                automatically.
              </p>
            </div>

            <div className="flex flex-wrap items-center gap-3">
              <button
                type="button"
                disabled
                aria-disabled="true"
                className="inline-flex cursor-not-allowed items-center gap-2 rounded-lg border border-fd-border bg-fd-muted px-4 py-2.5 text-sm font-medium text-fd-muted-foreground opacity-55"
              >
                <Download className="size-4" aria-hidden="true" />
                {acknowledged
                  ? 'No Release Published Yet'
                  : 'Acknowledge Warning to Download (No Release Yet)'}
              </button>

              <Link
                href="/docs/getting-started"
                className="inline-flex items-center gap-2 rounded-lg border border-fd-border bg-fd-secondary px-4 py-2.5 text-sm font-medium text-fd-secondary-foreground transition-colors hover:bg-fd-accent"
              >
                <Terminal className="size-4" aria-hidden="true" />
                Build from Source
              </Link>

              <a
                href={`${REPO_URL}/releases`}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1.5 text-sm font-medium text-fd-muted-foreground hover:text-fd-foreground"
              >
                GitHub Releases
                <ExternalLink className="size-3.5" aria-hidden="true" />
              </a>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
