// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import {
  AlertTriangle,
  CheckCircle2,
  Download,
  Info,
  ShieldCheck,
  Usb,
  XCircle,
} from 'lucide-react';
import {
  DUALSENSE_EDGE_PRODUCT_ID,
  DUALSENSE_PRODUCT_ID,
  REPORT_ID_FIRMWARE_INFO,
  REPORT_ID_SERIAL_COMMAND,
  REPORT_ID_SERIAL_RESPONSE,
  SONY_VENDOR_ID,
  type CompatibilityStatus,
  type ControllerCompatibility,
  type FirmwareInfoReport,
  type WebHidNavigator,
  decodeDualSenseColor,
  detectBluetoothConnection,
  parseFirmwareInfoReport,
  parseSerialResponse,
  resolveControllerCompatibility,
} from '@/lib/dualsense-webhid';

interface CheckResult {
  readonly vendorId: number;
  readonly productId: number;
  readonly isBluetooth: boolean;
  readonly firmwareInfo: FirmwareInfoReport;
  readonly compatibility: ControllerCompatibility;
  readonly shellColor: string | null;
}

interface CheckError {
  readonly isPermissionError: boolean;
  readonly title: string;
  readonly message: string;
}

function formatHex16(value: number): string {
  return `0x${(value & 0xffff).toString(16).toLowerCase().padStart(4, '0')}`;
}

function formatHex32Upper(value: number): string {
  return `0x${(value >>> 0).toString(16).toUpperCase().padStart(8, '0')}`;
}

function classifyError(error: unknown): CheckError {
  const name =
    error instanceof Error ||
    (typeof error === 'object' && error !== null && 'name' in error)
      ? String((error as { name?: unknown }).name ?? '')
      : '';
  const message =
    error instanceof Error
      ? error.message
      : typeof error === 'object' && error !== null && 'message' in error
        ? String((error as { message?: unknown }).message ?? '')
        : String(error);

  const combined = `${name} ${message}`;
  const isPermissionError =
    name === 'NotAllowedError' ||
    name === 'SecurityError' ||
    name === 'NetworkError' ||
    name === 'NotReadableError' ||
    /permission|access|open|udev|hidraw|denied|notallowed/i.test(combined);

  if (isPermissionError) {
    return {
      isPermissionError: true,
      title: 'Could not open controller over USB',
      message:
        'Your browser could not open the controller. On Linux, this usually means your user account needs hidraw udev permissions for Sony DualSense controllers, or another app has exclusive access.',
    };
  }

  return {
    isPermissionError: false,
    title: 'Could not read controller information',
    message:
      'Make sure your DualSense controller is connected directly with a data-capable USB-C cable (not a charge-only cable) and try again.',
  };
}

function StatusBadge({ status }: { status: CompatibilityStatus }) {
  if (status === 'supported') {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/40 bg-emerald-500/10 px-3 py-1 text-xs font-semibold text-emerald-600 dark:text-emerald-400">
        <CheckCircle2 className="size-3.5 shrink-0" aria-hidden="true" />
        <span>Supported</span>
      </span>
    );
  }

  if (status === 'untested') {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-full border border-amber-500/40 bg-amber-500/10 px-3 py-1 text-xs font-semibold text-amber-600 dark:text-amber-400">
        <AlertTriangle className="size-3.5 shrink-0" aria-hidden="true" />
        <span>Supported (Untested on Edge)</span>
      </span>
    );
  }

  return (
    <span className="inline-flex items-center gap-1.5 rounded-full border border-rose-500/40 bg-rose-500/10 px-3 py-1 text-xs font-semibold text-rose-600 dark:text-rose-400">
      <XCircle className="size-3.5 shrink-0" aria-hidden="true" />
      <span>Not Supported</span>
    </span>
  );
}

export function ControllerChecker() {
  const [webHidSupported, setWebHidSupported] = useState<boolean | null>(null);
  const [isChecking, setIsChecking] = useState(false);
  const [result, setResult] = useState<CheckResult | null>(null);
  const [errorState, setErrorState] = useState<CheckError | null>(null);

  useEffect(() => {
    const supported =
      typeof navigator !== 'undefined' &&
      'hid' in navigator &&
      Boolean((navigator as unknown as WebHidNavigator).hid);
    setWebHidSupported(supported);
  }, []);

  async function handleCheckController() {
    const hid = (navigator as unknown as WebHidNavigator).hid;
    if (!hid) {
      setWebHidSupported(false);
      return;
    }

    setIsChecking(true);
    setErrorState(null);

    try {
      const devices = await hid.requestDevice({
        filters: [
          { vendorId: SONY_VENDOR_ID, productId: DUALSENSE_PRODUCT_ID },
          { vendorId: SONY_VENDOR_ID, productId: DUALSENSE_EDGE_PRODUCT_ID },
        ],
      });

      if (devices.length === 0) {
        return;
      }

      const device = devices[0];
      try {
        if (!device.opened) {
          await device.open();
        }

        const isBluetooth = detectBluetoothConnection(device.collections ?? []);
        const reportView = await device.receiveFeatureReport(
          REPORT_ID_FIRMWARE_INFO,
        );
        const reportBytes = new Uint8Array(
          reportView.buffer,
          reportView.byteOffset,
          reportView.byteLength,
        );
        const firmwareInfo = parseFirmwareInfoReport(reportBytes);
        const compatibility = resolveControllerCompatibility(
          firmwareInfo,
          device.productId,
        );

        let shellColor: string | null = null;
        try {
          await device.sendFeatureReport(
            REPORT_ID_SERIAL_COMMAND,
            new Uint8Array([1, 19]),
          );
          const serialView = await device.receiveFeatureReport(
            REPORT_ID_SERIAL_RESPONSE,
          );
          const serialBytes = new Uint8Array(
            serialView.buffer,
            serialView.byteOffset,
            serialView.byteLength,
          );
          const serial = parseSerialResponse(serialBytes);
          if (serial) {
            shellColor = decodeDualSenseColor(serial);
          }
        } catch {
          // Shell color lookup via 0x80/0x81 is optional; ignore if blocked.
        }

        setResult({
          vendorId: device.vendorId,
          productId: device.productId,
          isBluetooth,
          firmwareInfo,
          compatibility,
          shellColor,
        });
      } finally {
        if (device.opened) {
          await device.close().catch(() => {});
        }
      }
    } catch (error) {
      setResult(null);
      setErrorState(classifyError(error));
    } finally {
      setIsChecking(false);
    }
  }

  const buildTimestamp = [
    result?.firmwareInfo.buildDate,
    result?.firmwareInfo.buildTime,
  ]
    .filter((part): part is string => Boolean(part && part.trim()))
    .join(' ');

  return (
    <div className="space-y-5 rounded-xl border border-fd-border bg-fd-card p-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="space-y-1">
          <p className="text-sm font-medium text-fd-card-foreground">
            Plug in your DualSense or DualSense Edge via USB-C, then run a quick
            read-only check.
          </p>
          <p className="flex items-start gap-2 text-xs leading-relaxed text-fd-muted-foreground">
            <ShieldCheck
              className="mt-0.5 size-4 shrink-0 text-fd-foreground"
              aria-hidden="true"
            />
            <span>
              Runs 100% locally in your browser: read-only check, nothing
              is sent to any server and nothing on your controller is changed.
            </span>
          </p>
        </div>

        <button
          type="button"
          onClick={handleCheckController}
          disabled={webHidSupported !== true || isChecking}
          className="inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 rounded-lg bg-fd-primary px-5 py-2.5 text-sm font-medium text-fd-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-55"
        >
          <Usb className="size-4 shrink-0" aria-hidden="true" />
          <span>{isChecking ? 'Checking Controller...' : 'Check My Controller'}</span>
        </button>
      </div>

      {webHidSupported === false ? (
        <div className="flex items-start gap-3 rounded-lg border border-fd-border bg-fd-background p-4 text-sm">
          <Info
            className="mt-0.5 size-4 shrink-0 text-fd-muted-foreground"
            aria-hidden="true"
          />
          <div className="space-y-2 text-fd-muted-foreground">
            <p className="font-medium text-fd-foreground">
              Browser USB check requires a Chromium desktop browser
            </p>
            <p className="leading-relaxed">
              Your current browser (such as Firefox, Safari, or a mobile
              browser) does not support WebHID. Try opening this page in desktop
              Chrome, Chromium, Edge, or Brave or simply use the{' '}
              <Link
                href="/download"
                className="font-medium text-fd-foreground underline underline-offset-4"
              >
                Linux desktop app
              </Link>
              , which checks controller compatibility automatically before any
              update.
            </p>
          </div>
        </div>
      ) : null}

      <div aria-live="polite" className="space-y-4">
        {!isChecking && errorState ? (
          <div className="flex items-start gap-3 rounded-lg border border-rose-500/40 bg-rose-500/10 p-4 text-sm">
            <XCircle
              className="mt-0.5 size-5 shrink-0 text-rose-500"
              aria-hidden="true"
            />
            <div className="space-y-2">
              <p className="font-semibold text-fd-foreground">
                {errorState.title}
              </p>
              <p className="leading-relaxed text-fd-muted-foreground">
                {errorState.message}
              </p>
              <p>
                <Link
                  href="/docs/safety-and-troubleshooting#linux-usb-permissions-udev"
                  className="font-medium text-fd-foreground underline underline-offset-4"
                >
                  {errorState.isPermissionError
                    ? 'Open the Linux USB permissions (udev) guide'
                    : 'Read USB connection & udev troubleshooting'}
                </Link>
              </p>
            </div>
          </div>
        ) : null}

        {!isChecking && result ? (
          <div className="space-y-4 rounded-xl border border-fd-border bg-fd-background p-5">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="space-y-1">
                <h3 className="text-lg font-semibold text-fd-foreground">
                  {result.compatibility.modelLabel}
                  {result.shellColor ? ` — ${result.shellColor}` : ''}
                </h3>
                <p className="text-sm text-fd-muted-foreground">
                  {result.compatibility.summary}
                </p>
              </div>
              <StatusBadge status={result.compatibility.status} />
            </div>

            {result.isBluetooth ? (
              <div className="flex items-start gap-2.5 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3.5 text-sm">
                <AlertTriangle
                  className="mt-0.5 size-4 shrink-0 text-amber-500"
                  aria-hidden="true"
                />
                <p className="leading-relaxed text-fd-muted-foreground">
                  <strong className="text-fd-foreground">
                    Bluetooth connection detected:
                  </strong>{' '}
                  Firmware updates can only run over a wired USB-C data cable.
                  Disconnect Bluetooth and plug in directly with a USB-C cable
                  before running the Linux updater.
                </p>
              </div>
            ) : null}

            {result.compatibility.reason ? (
              <p className="text-sm leading-relaxed text-fd-muted-foreground">
                {result.compatibility.reason}
              </p>
            ) : null}

            <dl className="grid grid-cols-1 gap-3 border-t border-fd-border pt-4 text-sm sm:grid-cols-3">
              <div>
                <dt className="text-xs text-fd-muted-foreground">
                  Current Firmware
                </dt>
                <dd className="font-mono font-medium text-fd-foreground">
                  {result.firmwareInfo.firmwareVersionHex}
                </dd>
              </div>
              <div>
                <dt className="text-xs text-fd-muted-foreground">Connection</dt>
                <dd className="font-medium text-fd-foreground">
                  {result.isBluetooth
                    ? 'Bluetooth (USB-C required to update)'
                    : 'USB'}
                </dd>
              </div>
              {result.shellColor ? (
                <div>
                  <dt className="text-xs text-fd-muted-foreground">
                    Shell Color
                  </dt>
                  <dd className="font-medium text-fd-foreground">
                    {result.shellColor}
                  </dd>
                </div>
              ) : null}
            </dl>

            <div className="flex flex-wrap items-center gap-3 pt-1">
              {result.compatibility.status === 'supported' ||
              result.compatibility.status === 'untested' ? (
                <Link
                  href="/download"
                  className="inline-flex items-center gap-2 rounded-lg bg-fd-primary px-4 py-2 text-sm font-medium text-fd-primary-foreground transition-opacity hover:opacity-90"
                >
                  <Download className="size-4 shrink-0" aria-hidden="true" />
                  <span>Download Linux Updater</span>
                </Link>
              ) : (
                <Link
                  href="/docs/safety-and-troubleshooting"
                  className="inline-flex items-center gap-2 rounded-lg border border-fd-border bg-fd-card px-4 py-2 text-sm font-medium text-fd-foreground transition-colors hover:bg-fd-accent"
                >
                  <span>Read Safety and Troubleshooting</span>
                </Link>
              )}
            </div>

            <details className="rounded-lg border border-fd-border bg-fd-card p-3.5 text-xs">
              <summary className="cursor-pointer font-medium text-fd-foreground">
                Technical hardware details
              </summary>
              <dl className="mt-3 grid grid-cols-1 gap-x-4 gap-y-2.5 sm:grid-cols-2">
                <div>
                  <dt className="text-fd-muted-foreground">Board Revision</dt>
                  <dd className="font-mono text-fd-foreground">
                    {result.compatibility.boardName}
                  </dd>
                </div>
                <div>
                  <dt className="text-fd-muted-foreground">
                    Sony Firmware Target
                  </dt>
                  <dd className="font-mono text-fd-foreground">
                    {result.compatibility.targetId ?? 'None'}
                  </dd>
                </div>
                <div>
                  <dt className="text-fd-muted-foreground">USB VID:PID</dt>
                  <dd className="font-mono text-fd-foreground">
                    {formatHex16(result.vendorId)}:
                    {formatHex16(result.productId)}
                  </dd>
                </div>
                <div>
                  <dt className="text-fd-muted-foreground">Firmware Series</dt>
                  <dd className="font-mono text-fd-foreground">
                    {result.firmwareInfo.fwSeriesHex}
                  </dd>
                </div>
                <div>
                  <dt className="text-fd-muted-foreground">Hardware Info</dt>
                  <dd className="font-mono text-fd-foreground">
                    {formatHex32Upper(result.firmwareInfo.hwInfo)}
                  </dd>
                </div>
                <div>
                  <dt className="text-fd-muted-foreground">Firmware Build</dt>
                  <dd className="font-mono text-fd-foreground">
                    {buildTimestamp || 'Unavailable'}
                  </dd>
                </div>
              </dl>
            </details>
          </div>
        ) : null}
      </div>
    </div>
  );
}
