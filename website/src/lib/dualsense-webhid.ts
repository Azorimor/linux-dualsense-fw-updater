// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

export const SONY_VENDOR_ID = 0x054c;
export const DUALSENSE_PRODUCT_ID = 0x0ce6;
export const DUALSENSE_EDGE_PRODUCT_ID = 0x0df2;

export const REPORT_ID_FIRMWARE_INFO = 0x20;
export const REPORT_ID_SERIAL_COMMAND = 0x80;
export const REPORT_ID_SERIAL_RESPONSE = 0x81;

const FIRMWARE_INFO_REPORT_LEN = 64;

export interface WebHidReportItem {
  readonly reportSize?: number;
  readonly reportCount?: number;
}

export interface WebHidReportInfo {
  readonly reportId?: number;
  readonly items?: readonly WebHidReportItem[];
}

export interface WebHidCollectionInfo {
  readonly usagePage?: number;
  readonly usage?: number;
  readonly type?: number;
  readonly inputReports?: readonly WebHidReportInfo[];
  readonly outputReports?: readonly WebHidReportInfo[];
  readonly featureReports?: readonly WebHidReportInfo[];
  readonly children?: readonly WebHidCollectionInfo[];
}

export interface WebHidDeviceFilter {
  readonly vendorId?: number;
  readonly productId?: number;
  readonly usagePage?: number;
  readonly usage?: number;
}

export interface WebHidDevice {
  readonly opened: boolean;
  readonly vendorId: number;
  readonly productId: number;
  readonly productName?: string;
  readonly collections?: readonly WebHidCollectionInfo[];
  open(): Promise<void>;
  close(): Promise<void>;
  receiveFeatureReport(reportId: number): Promise<DataView>;
  sendFeatureReport(reportId: number, data: Uint8Array): Promise<void>;
}

export interface WebHidNavigator {
  readonly hid?: {
    requestDevice(options: {
      filters: readonly WebHidDeviceFilter[];
    }): Promise<WebHidDevice[]>;
  };
}

export type BoardModelName =
  | 'BDM-010'
  | 'BDM-020'
  | 'BDM-030'
  | 'BDM-040'
  | 'BDM-050'
  | 'BDM-060R'
  | 'BDM-060M'
  | 'BDM-060X'
  | 'Unknown';

export type SonyTargetId = '0004' | '000B' | '000E' | '0044';

export interface DecodedBoardModel {
  readonly boardName: BoardModelName;
  readonly targetId: SonyTargetId | null;
}

export interface FirmwareInfoReport {
  readonly buildDate: string;
  readonly buildTime: string;
  readonly fwType: number;
  readonly fwSeries: number;
  readonly fwSeriesHex: string;
  readonly hwInfo: number;
  readonly fwVersion: number;
  readonly firmwareVersion: number;
  readonly firmwareVersionHex: string;
  readonly boardModel: BoardModelName;
}

export type CompatibilityStatus = 'supported' | 'untested' | 'unsupported';
export type ControllerModelLabel = 'DualSense' | 'DualSense Edge';

export interface ControllerCompatibility {
  readonly status: CompatibilityStatus;
  readonly modelLabel: ControllerModelLabel;
  readonly boardName: string;
  readonly targetId: SonyTargetId | null;
  readonly summary: string;
  readonly reason?: string;
}

const COLOR_CODES: Readonly<Record<string, string>> = {
  '00': 'White',
  '01': 'Midnight Black',
  '02': 'Cosmic Red',
  '03': 'Nova Pink',
  '04': 'Galactic Purple',
  '05': 'Starlight Blue',
  '06': 'Grey Camouflage',
  '07': 'Volcanic Red',
  '08': 'Sterling Silver',
  '09': 'Cobalt Blue',
  '10': 'Chroma Teal',
  '11': 'Chroma Indigo',
  '12': 'Chroma Pearl',
  '13': 'HyperPop Techno Red',
  '14': 'HyperPop Remix Green',
  '15': 'HyperPop Rhythm Blue',
  '30': '30th Anniversary',
  Z1: 'God of War Ragnarok',
  Z2: 'Spider-Man 2',
  Z3: 'Astro Bot',
  Z4: 'Fortnite',
  Z6: 'The Last of Us',
  ZA: 'God of War 20th Anniversary',
  ZB: 'Icon Blue Limited Edition',
  ZC: 'Ghost of Yotei Limited Edition',
  ZD: 'Marathon Limited Edition',
  ZE: 'Genshin Impact Limited Edition',
  ZF: '007 First Light Limited Edition',
};

const utf8LossyDecoder = new TextDecoder('utf-8', { fatal: false });

export function decodeAscii(data: Uint8Array): string {
  const nulIndex = data.indexOf(0);
  const slice = nulIndex === -1 ? data : data.subarray(0, nulIndex);
  return utf8LossyDecoder.decode(slice);
}

function formatHex16Upper(value: number): string {
  return `0x${(value & 0xffff).toString(16).toUpperCase().padStart(4, '0')}`;
}

function formatHex16Lower(value: number): string {
  return `0x${(value & 0xffff).toString(16).toLowerCase().padStart(4, '0')}`;
}

function formatHex32Upper(value: number): string {
  return `0x${(value >>> 0).toString(16).toUpperCase().padStart(8, '0')}`;
}

export function decodeBoardModel(hwInfo: number): DecodedBoardModel {
  const boardByte = (hwInfo >>> 8) & 0xff;
  switch (boardByte) {
    case 0x03:
      return { boardName: 'BDM-010', targetId: null };
    case 0x04:
      return { boardName: 'BDM-020', targetId: '0004' };
    case 0x05:
      return { boardName: 'BDM-030', targetId: '0004' };
    case 0x06:
      return { boardName: 'BDM-040', targetId: '0004' };
    case 0x07:
    case 0x08:
      return { boardName: 'BDM-050', targetId: '000B' };
    case 0x09:
      return { boardName: 'BDM-060R', targetId: '000E' };
    case 0x11:
      return { boardName: 'BDM-060M', targetId: '000E' };
    case 0x13:
      return { boardName: 'BDM-060X', targetId: '000E' };
    default:
      return { boardName: 'Unknown', targetId: null };
  }
}

export function targetIdForFwSeries(fwSeries: number): SonyTargetId | null {
  switch (fwSeries & 0xffff) {
    case 0x0004:
      return '0004';
    case 0x000b:
      return '000B';
    case 0x000e:
      return '000E';
    case 0x0044:
      return '0044';
    default:
      return null;
  }
}

export function parseFirmwareInfoReport(raw: Uint8Array): FirmwareInfoReport {
  if (raw.length < 20) {
    throw new Error(
      `Firmware info report too short: expected at least 20 bytes, got ${raw.length}`,
    );
  }

  let payload: Uint8Array;
  let hasLeadingReportId: boolean;

  if (raw.length === 63) {
    // WebHID frequently strips the leading 0x20 report ID; normalize to 64 bytes.
    const normalized = new Uint8Array(FIRMWARE_INFO_REPORT_LEN);
    normalized[0] = REPORT_ID_FIRMWARE_INFO;
    normalized.set(raw, 1);
    payload = normalized;
    hasLeadingReportId = true;
  } else if (raw.length === 65 && raw[0] === REPORT_ID_FIRMWARE_INFO) {
    payload = raw.subarray(1);
    hasLeadingReportId = false;
  } else if (
    raw.length >= FIRMWARE_INFO_REPORT_LEN &&
    raw[0] === REPORT_ID_FIRMWARE_INFO
  ) {
    payload = raw;
    hasLeadingReportId = true;
  } else {
    payload = raw;
    hasLeadingReportId = false;
  }

  if (payload.length < 47) {
    throw new Error(
      `Firmware info payload too short: expected at least 47 bytes, got ${payload.length}`,
    );
  }

  const buildDate = decodeAscii(
    hasLeadingReportId ? payload.subarray(1, 12) : payload.subarray(0, 12),
  );
  const buildTime = decodeAscii(payload.subarray(12, 20));

  const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
  const fwType = view.getUint16(20, true);
  const fwSeries = view.getUint16(22, true);
  const hwInfo = view.getUint32(24, true);
  const fwVersion = view.getUint32(28, true);
  const firmwareVersion = view.getUint16(44, true);

  return {
    buildDate,
    buildTime,
    fwType,
    fwSeries,
    fwSeriesHex: formatHex16Upper(fwSeries),
    hwInfo,
    fwVersion,
    firmwareVersion,
    firmwareVersionHex: formatHex16Lower(firmwareVersion),
    boardModel: decodeBoardModel(hwInfo).boardName,
  };
}

export function resolveControllerCompatibility(
  info: FirmwareInfoReport,
  productId: number,
): ControllerCompatibility {
  const seriesTarget = targetIdForFwSeries(info.fwSeries);

  if (productId === DUALSENSE_EDGE_PRODUCT_ID) {
    if (info.fwSeries === 0 || info.fwSeries === 0x0044) {
      return {
        status: 'untested',
        modelLabel: 'DualSense Edge',
        boardName: 'DualSense Edge',
        targetId: '0044',
        summary:
          'DualSense Edge is listed in Sony’s official firmware catalog, though flashing has not yet been tested on DualSense Edge hardware.',
      };
    }

    const reason = `This DualSense Edge reported firmware series ${info.fwSeriesHex}, which does not match Sony's DualSense Edge update target (0044).`;
    return {
      status: 'unsupported',
      modelLabel: 'DualSense Edge',
      boardName: 'DualSense Edge',
      targetId: null,
      summary: 'DualSense Edge reported an unexpected firmware series.',
      reason,
    };
  }

  const decoded = decodeBoardModel(info.hwInfo);
  const boardName = decoded.boardName;

  if (decoded.targetId !== null) {
    if (info.fwSeries === 0 || seriesTarget === decoded.targetId) {
      return {
        status: 'supported',
        modelLabel: 'DualSense',
        boardName,
        targetId: decoded.targetId,
        summary:
          'Your DualSense controller is supported by the Linux firmware updater.',
      };
    }

    if (seriesTarget === null) {
      const reason = `This ${boardName} controller reported firmware series ${info.fwSeriesHex}, which is not in Sony's official update catalog.`;
      return {
        status: 'unsupported',
        modelLabel: 'DualSense',
        boardName,
        targetId: null,
        summary:
          'This controller reported an unrecognized firmware series and cannot be updated automatically.',
        reason,
      };
    }

    const reason = `Controller board ${boardName} does not match reported firmware series ${info.fwSeriesHex}.`;
    return {
      status: 'unsupported',
      modelLabel: 'DualSense',
      boardName,
      targetId: null,
      summary:
        'Controller board revision does not match its reported firmware series.',
      reason,
    };
  }

  if (decoded.boardName === 'BDM-010') {
    const reason =
      'Original 2020 launch DualSense controllers (BDM-010) are not included in Sony’s official PC firmware catalog. Please update this controller using a PlayStation 5 console.';
    return {
      status: 'unsupported',
      modelLabel: 'DualSense',
      boardName: 'BDM-010',
      targetId: null,
      summary:
        'Original 2020 launch DualSense models are not supported by Sony’s PC firmware catalog — update via PlayStation 5.',
      reason,
    };
  }

  if (info.hwInfo === 0 && info.fwSeries === 0) {
    const reason =
      'The controller did not report a hardware board revision or firmware series.';
    return {
      status: 'unsupported',
      modelLabel: 'DualSense',
      boardName: 'Unknown',
      targetId: null,
      summary: 'Could not identify the controller hardware revision.',
      reason,
    };
  }

  if (seriesTarget !== null) {
    if (seriesTarget === '0044') {
      const reason = `Controller reported DualSense Edge firmware series ${info.fwSeriesHex}, but is not a DualSense Edge USB device.`;
      return {
        status: 'unsupported',
        modelLabel: 'DualSense',
        boardName: 'Unknown',
        targetId: null,
        summary:
          'Controller reported DualSense Edge firmware on a standard DualSense USB device.',
        reason,
      };
    }

    return {
      status: 'supported',
      modelLabel: 'DualSense',
      boardName: 'Unknown',
      targetId: seriesTarget,
      summary:
        'Your DualSense controller matches an official Sony firmware series and is supported.',
    };
  }

  const reason = `Unrecognized controller hardware (${formatHex32Upper(info.hwInfo)}, firmware series ${info.fwSeriesHex}) is not in Sony's official update catalog.`;
  return {
    status: 'unsupported',
    modelLabel: 'DualSense',
    boardName: 'Unknown',
    targetId: null,
    summary:
      'This controller hardware revision is not listed in Sony’s official update catalog.',
    reason,
  };
}

function getReportTotalBits(report: WebHidReportInfo): number {
  if (!report.items || report.items.length === 0) {
    return 0;
  }
  let totalBits = 0;
  for (const item of report.items) {
    const size = item.reportSize ?? 0;
    const count = item.reportCount ?? 0;
    if (size > 0 && count > 0) {
      totalBits += size * count;
    }
  }
  return totalBits;
}

function hasBluetoothInputReport(collection: WebHidCollectionInfo): boolean {
  if (collection.inputReports) {
    for (const report of collection.inputReports) {
      if (report.reportId === 0x31) {
        return true;
      }
      if (report.reportId === 0x01) {
        const totalBits = getReportTotalBits(report);
        if (totalBits > 0 && totalBits < 256) {
          return true;
        }
      }
    }
  }
  if (collection.children) {
    for (const child of collection.children) {
      if (hasBluetoothInputReport(child)) {
        return true;
      }
    }
  }
  return false;
}

export function detectBluetoothConnection(
  collections: readonly WebHidCollectionInfo[],
): boolean {
  for (const collection of collections) {
    if (hasBluetoothInputReport(collection)) {
      return true;
    }
  }
  return false;
}

export function parseSerialResponse(raw: Uint8Array): string | null {
  let serialSlice: Uint8Array;

  if (
    raw.length >= 21 &&
    raw[0] === REPORT_ID_SERIAL_RESPONSE &&
    raw[1] === 1 &&
    raw[2] === 19 &&
    raw[3] === 2
  ) {
    serialSlice = raw.subarray(4, 21);
  } else if (
    raw.length >= 20 &&
    raw[0] === 1 &&
    raw[1] === 19 &&
    raw[2] === 2
  ) {
    serialSlice = raw.subarray(3, 20);
  } else {
    return null;
  }

  const serial = decodeAscii(serialSlice).trim();
  return serial.length > 0 ? serial : null;
}

export function decodeDualSenseColor(serial: string): string | null {
  if (!/^[\x00-\x7f]*$/.test(serial) || serial.length < 6) {
    return null;
  }
  const code = serial.slice(4, 6).toUpperCase();
  return COLOR_CODES[code] ?? null;
}
