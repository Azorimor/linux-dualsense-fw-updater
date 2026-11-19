// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import assert from 'node:assert/strict';
import test from 'node:test';
import {
  DUALSENSE_EDGE_PRODUCT_ID,
  DUALSENSE_PRODUCT_ID,
  REPORT_ID_FIRMWARE_INFO,
  REPORT_ID_SERIAL_COMMAND,
  REPORT_ID_SERIAL_RESPONSE,
  SONY_VENDOR_ID,
  type FirmwareInfoReport,
  type WebHidCollectionInfo,
  decodeAscii,
  decodeBoardModel,
  decodeDualSenseColor,
  detectBluetoothConnection,
  parseFirmwareInfoReport,
  parseSerialResponse,
  resolveControllerCompatibility,
} from './dualsense-webhid.ts';

function writeAscii(buf: Uint8Array, offset: number, text: string): void {
  for (let i = 0; i < text.length; i += 1) {
    buf[offset + i] = text.charCodeAt(i);
  }
}

function writeU16LE(buf: Uint8Array, offset: number, value: number): void {
  const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  view.setUint16(offset, value, true);
}

function writeU32LE(buf: Uint8Array, offset: number, value: number): void {
  const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  view.setUint32(offset, value, true);
}

function makeFirmwareInfo(
  overrides: Partial<FirmwareInfoReport> = {},
): FirmwareInfoReport {
  return {
    buildDate: 'Jul 18 2025',
    buildTime: '15:42:10',
    fwType: 0x0003,
    fwSeries: 0x000b,
    fwSeriesHex: '0x000B',
    hwInfo: 0x00000811,
    fwVersion: 0x00010630,
    firmwareVersion: 0x0630,
    firmwareVersionHex: '0x0630',
    boardModel: 'BDM-050',
    ...overrides,
  };
}

test('constants match DualSense USB and HID report identifiers', () => {
  assert.equal(SONY_VENDOR_ID, 0x054c);
  assert.equal(DUALSENSE_PRODUCT_ID, 0x0ce6);
  assert.equal(DUALSENSE_EDGE_PRODUCT_ID, 0x0df2);
  assert.equal(REPORT_ID_FIRMWARE_INFO, 0x20);
  assert.equal(REPORT_ID_SERIAL_COMMAND, 0x80);
  assert.equal(REPORT_ID_SERIAL_RESPONSE, 0x81);
});

test('decodeAscii stops at first NUL byte and handles non-UTF8 safely', () => {
  assert.equal(
    decodeAscii(new TextEncoder().encode('Jan 01 2026\0extra')),
    'Jan 01 2026',
  );
  assert.equal(decodeAscii(new Uint8Array([0, 65, 66])), '');
  assert.equal(decodeAscii(new Uint8Array([65, 0xff, 66, 0])), 'A\uFFFDB');
});

test('parseFirmwareInfoReport handles 63-byte stripped WebHID reports', () => {
  const raw63 = new Uint8Array(63);
  writeAscii(raw63, 0, 'Jul 18 2025');
  writeAscii(raw63, 11, '15:42:10');
  writeU16LE(raw63, 19, 0x0003);
  writeU16LE(raw63, 21, 0x000b);
  writeU32LE(raw63, 23, 0x00000811);
  writeU32LE(raw63, 27, 0x00010630);
  writeU16LE(raw63, 43, 0x0630);

  const info = parseFirmwareInfoReport(raw63);
  assert.equal(info.buildDate, 'Jul 18 2025');
  assert.equal(info.buildTime, '15:42:10');
  assert.equal(info.fwType, 0x0003);
  assert.equal(info.fwSeries, 0x000b);
  assert.equal(info.fwSeriesHex, '0x000B');
  assert.equal(info.hwInfo, 0x00000811);
  assert.equal(info.fwVersion, 0x00010630);
  assert.equal(info.firmwareVersion, 0x0630);
  assert.equal(info.firmwareVersionHex, '0x0630');
  assert.equal(info.boardModel, 'BDM-050');
});

test('parseFirmwareInfoReport handles 64-byte report with leading 0x20 and 68-byte Bluetooth report', () => {
  const raw64 = new Uint8Array(64);
  raw64[0] = REPORT_ID_FIRMWARE_INFO;
  writeAscii(raw64, 1, 'Jul 18 2025');
  writeAscii(raw64, 12, '15:42:10');
  writeU16LE(raw64, 20, 0x0003);
  writeU16LE(raw64, 22, 0x000b);
  writeU32LE(raw64, 24, 0x00000811);
  writeU32LE(raw64, 28, 0x00010630);
  writeU16LE(raw64, 44, 0x064f);

  const info64 = parseFirmwareInfoReport(raw64);
  assert.equal(info64.buildDate, 'Jul 18 2025');
  assert.equal(info64.buildTime, '15:42:10');
  assert.equal(info64.fwType, 0x0003);
  assert.equal(info64.fwSeries, 0x000b);
  assert.equal(info64.fwSeriesHex, '0x000B');
  assert.equal(info64.hwInfo, 0x00000811);
  assert.equal(info64.fwVersion, 0x00010630);
  assert.equal(info64.firmwareVersion, 0x064f);
  assert.equal(info64.firmwareVersionHex, '0x064f');
  assert.equal(info64.boardModel, 'BDM-050');

  const raw68 = new Uint8Array(68);
  raw68.set(raw64, 0);
  raw68[64] = 0xde;
  raw68[65] = 0xad;
  raw68[66] = 0xbe;
  raw68[67] = 0xef;

  const info68 = parseFirmwareInfoReport(raw68);
  assert.deepEqual(info68, info64);

  // Verify non-zero byteOffset subarrays work properly
  const padded = new Uint8Array(80);
  padded.set(raw64, 8);
  const sub = padded.subarray(8, 72);
  assert.deepEqual(parseFirmwareInfoReport(sub), info64);
});

test('parseFirmwareInfoReport handles 64-byte payload without leading 0x20 and 65-byte report with leading 0x20', () => {
  const payload64 = new Uint8Array(64);
  writeAscii(payload64, 0, 'Jan 15 2025');
  writeAscii(payload64, 12, '10:20:30');
  writeU16LE(payload64, 20, 0x0002);
  writeU16LE(payload64, 22, 0x0004);
  writeU32LE(payload64, 24, 0x00000500);
  writeU32LE(payload64, 28, 0x00010520);
  writeU16LE(payload64, 44, 0x0520);

  const infoPayload = parseFirmwareInfoReport(payload64);
  assert.equal(infoPayload.buildDate, 'Jan 15 2025');
  assert.equal(infoPayload.buildTime, '10:20:30');
  assert.equal(infoPayload.fwType, 0x0002);
  assert.equal(infoPayload.fwSeries, 0x0004);
  assert.equal(infoPayload.hwInfo, 0x00000500);
  assert.equal(infoPayload.fwVersion, 0x00010520);
  assert.equal(infoPayload.firmwareVersion, 0x0520);
  assert.equal(infoPayload.firmwareVersionHex, '0x0520');
  assert.equal(infoPayload.boardModel, 'BDM-030');

  const raw65 = new Uint8Array(65);
  raw65[0] = REPORT_ID_FIRMWARE_INFO;
  writeAscii(raw65, 1, 'Sep 01 2025');
  writeAscii(raw65, 13, '08:00:00');
  writeU16LE(raw65, 21, 0x0001);
  writeU16LE(raw65, 23, 0x000e);
  writeU32LE(raw65, 25, 0x00001100);
  writeU32LE(raw65, 29, 0x00020641);
  writeU16LE(raw65, 45, 0x0641);

  const info65 = parseFirmwareInfoReport(raw65);
  assert.equal(info65.buildDate, 'Sep 01 2025');
  assert.equal(info65.buildTime, '08:00:00');
  assert.equal(info65.fwType, 0x0001);
  assert.equal(info65.fwSeries, 0x000e);
  assert.equal(info65.fwSeriesHex, '0x000E');
  assert.equal(info65.hwInfo, 0x00001100);
  assert.equal(info65.fwVersion, 0x00020641);
  assert.equal(info65.firmwareVersion, 0x0641);
  assert.equal(info65.firmwareVersionHex, '0x0641');
  assert.equal(info65.boardModel, 'BDM-060M');
});

test('parseFirmwareInfoReport rejects short reports and short payloads', () => {
  assert.throws(
    () => parseFirmwareInfoReport(new Uint8Array(0)),
    /at least 20 bytes/,
  );
  assert.throws(
    () => parseFirmwareInfoReport(new Uint8Array(19)),
    /at least 20 bytes/,
  );
  assert.throws(
    () => parseFirmwareInfoReport(new Uint8Array(46)),
    /at least 47 bytes/,
  );
});

test('decodeBoardModel decodes all known DualSense revisions and unknown fallback', () => {
  const cases: Array<[number, string, string | null]> = [
    [0x00000300, 'BDM-010', null],
    [0x00000411, 'BDM-020', '0004'],
    [0x00000500, 'BDM-030', '0004'],
    [0x00000600, 'BDM-040', '0004'],
    [0x00000700, 'BDM-050', '000B'],
    [0x00000811, 'BDM-050', '000B'],
    [0x00000900, 'BDM-060R', '000E'],
    [0x00001100, 'BDM-060M', '000E'],
    [0x00001300, 'BDM-060X', '000E'],
    [0x00000000, 'Unknown', null],
    [0x0000ff00, 'Unknown', null],
  ];

  for (const [hwInfo, expectedBoard, expectedTarget] of cases) {
    const decoded = decodeBoardModel(hwInfo);
    assert.equal(decoded.boardName, expectedBoard);
    assert.equal(decoded.targetId, expectedTarget);
  }
});

test('resolveControllerCompatibility matches Rust target resolution across boards, Edge, BDM-010, and fallbacks', () => {
  // Supported BDM-020..060
  const supportedCases: Array<[number, number, string, string]> = [
    [0x00000400, 0x0004, 'BDM-020', '0004'],
    [0x00000500, 0x0004, 'BDM-030', '0004'],
    [0x00000600, 0x0004, 'BDM-040', '0004'],
    [0x00000700, 0x0000, 'BDM-050', '000B'],
    [0x00000811, 0x000b, 'BDM-050', '000B'],
    [0x00000900, 0x000e, 'BDM-060R', '000E'],
    [0x00001100, 0x000e, 'BDM-060M', '000E'],
    [0x00001300, 0x000e, 'BDM-060X', '000E'],
  ];

  for (const [hwInfo, fwSeries, boardName, targetId] of supportedCases) {
    const res = resolveControllerCompatibility(
      makeFirmwareInfo({
        hwInfo,
        fwSeries,
        fwSeriesHex: `0x${fwSeries.toString(16).toUpperCase().padStart(4, '0')}`,
      }),
      DUALSENSE_PRODUCT_ID,
    );
    assert.equal(res.status, 'supported');
    assert.equal(res.modelLabel, 'DualSense');
    assert.equal(res.boardName, boardName);
    assert.equal(res.targetId, targetId);
    assert.equal(res.reason, undefined);
  }

  // DualSense Edge (0x0044 and zeroed fwSeries -> untested)
  for (const fwSeries of [0x0044, 0x0000]) {
    const edge = resolveControllerCompatibility(
      makeFirmwareInfo({
        hwInfo: 0x00000400,
        fwSeries,
        fwSeriesHex: `0x${fwSeries.toString(16).toUpperCase().padStart(4, '0')}`,
      }),
      DUALSENSE_EDGE_PRODUCT_ID,
    );
    assert.equal(edge.status, 'untested');
    assert.equal(edge.modelLabel, 'DualSense Edge');
    assert.equal(edge.boardName, 'DualSense Edge');
    assert.equal(edge.targetId, '0044');
  }

  // DualSense Edge with mismatched fwSeries -> unsupported
  const edgeMismatch = resolveControllerCompatibility(
    makeFirmwareInfo({ fwSeries: 0x0004, fwSeriesHex: '0x0004' }),
    DUALSENSE_EDGE_PRODUCT_ID,
  );
  assert.equal(edgeMismatch.status, 'unsupported');
  assert.equal(edgeMismatch.targetId, null);
  assert.match(edgeMismatch.reason ?? '', /0x0004/);

  // BDM-010 -> unsupported with PS5 / 2020 guidance
  const bdm010 = resolveControllerCompatibility(
    makeFirmwareInfo({
      hwInfo: 0x00000300,
      fwSeries: 0x0001,
      fwSeriesHex: '0x0001',
      boardModel: 'BDM-010',
    }),
    DUALSENSE_PRODUCT_ID,
  );
  assert.equal(bdm010.status, 'unsupported');
  assert.equal(bdm010.boardName, 'BDM-010');
  assert.equal(bdm010.targetId, null);
  assert.match(bdm010.reason ?? '', /2020/);
  assert.match(bdm010.reason ?? '', /PlayStation 5/);

  // Known board with unknown non-zero fwSeries -> unsupported
  const badSeries = resolveControllerCompatibility(
    makeFirmwareInfo({
      hwInfo: 0x00000811,
      fwSeries: 0x0077,
      fwSeriesHex: '0x0077',
    }),
    DUALSENSE_PRODUCT_ID,
  );
  assert.equal(badSeries.status, 'unsupported');
  assert.equal(badSeries.targetId, null);

  // Known board with mismatched known fwSeries -> unsupported
  const mismatchSeries = resolveControllerCompatibility(
    makeFirmwareInfo({
      hwInfo: 0x00000811,
      fwSeries: 0x0004,
      fwSeriesHex: '0x0004',
    }),
    DUALSENSE_PRODUCT_ID,
  );
  assert.equal(mismatchSeries.status, 'unsupported');
  assert.equal(mismatchSeries.targetId, null);

  // Unknown board with zeroed hwInfo and zeroed fwSeries -> unsupported
  const zeroed = resolveControllerCompatibility(
    makeFirmwareInfo({
      hwInfo: 0,
      fwSeries: 0,
      fwSeriesHex: '0x0000',
      boardModel: 'Unknown',
    }),
    DUALSENSE_PRODUCT_ID,
  );
  assert.equal(zeroed.status, 'unsupported');
  assert.equal(zeroed.boardName, 'Unknown');
  assert.equal(zeroed.targetId, null);

  // Unknown board with known standard fwSeries -> supported fallback
  const unknownKnownSeries = resolveControllerCompatibility(
    makeFirmwareInfo({
      hwInfo: 0x00001900,
      fwSeries: 0x000e,
      fwSeriesHex: '0x000E',
      boardModel: 'Unknown',
    }),
    DUALSENSE_PRODUCT_ID,
  );
  assert.equal(unknownKnownSeries.status, 'supported');
  assert.equal(unknownKnownSeries.boardName, 'Unknown');
  assert.equal(unknownKnownSeries.targetId, '000E');

  // Unknown board with Edge fwSeries 0x0044 on non-Edge PID -> unsupported
  const nonEdge0044 = resolveControllerCompatibility(
    makeFirmwareInfo({
      hwInfo: 0x00001f00,
      fwSeries: 0x0044,
      fwSeriesHex: '0x0044',
      boardModel: 'Unknown',
    }),
    DUALSENSE_PRODUCT_ID,
  );
  assert.equal(nonEdge0044.status, 'unsupported');
  assert.equal(nonEdge0044.targetId, null);

  // Unknown board with unrecognized fwSeries -> unsupported
  const unknownBad = resolveControllerCompatibility(
    makeFirmwareInfo({
      hwInfo: 0x00001f00,
      fwSeries: 0x0099,
      fwSeriesHex: '0x0099',
      boardModel: 'Unknown',
    }),
    DUALSENSE_PRODUCT_ID,
  );
  assert.equal(unknownBad.status, 'unsupported');
  assert.equal(unknownBad.targetId, null);
});

test('detectBluetoothConnection identifies Bluetooth descriptors and accepts USB descriptors', () => {
  const usbCollections: WebHidCollectionInfo[] = [
    {
      inputReports: [
        {
          reportId: 0x01,
          items: [{ reportSize: 8, reportCount: 63 }],
        },
      ],
    },
  ];
  assert.equal(detectBluetoothConnection(usbCollections), false);

  const btExtendedCollections: WebHidCollectionInfo[] = [
    {
      inputReports: [
        {
          reportId: 0x31,
          items: [{ reportSize: 8, reportCount: 77 }],
        },
      ],
    },
  ];
  assert.equal(detectBluetoothConnection(btExtendedCollections), true);

  const btSimpleCollections: WebHidCollectionInfo[] = [
    {
      inputReports: [
        {
          reportId: 0x01,
          items: [{ reportSize: 8, reportCount: 9 }],
        },
      ],
    },
  ];
  assert.equal(detectBluetoothConnection(btSimpleCollections), true);

  const btNestedCollections: WebHidCollectionInfo[] = [
    {
      children: [
        {
          inputReports: [{ reportId: 0x31 }],
        },
      ],
    },
  ];
  assert.equal(detectBluetoothConnection(btNestedCollections), true);

  assert.equal(detectBluetoothConnection([]), false);
  assert.equal(
    detectBluetoothConnection([
      {
        inputReports: [{ reportId: 0x01, items: [] }],
      },
    ]),
    false,
  );
});

test('parseSerialResponse supports 0x81-prefixed and 63-byte stripped buffers and rejects malformed frames', () => {
  const withPrefix = new Uint8Array(64);
  withPrefix[0] = REPORT_ID_SERIAL_RESPONSE;
  withPrefix[1] = 1;
  withPrefix[2] = 19;
  withPrefix[3] = 2;
  writeAscii(withPrefix, 4, 'F12301ABCDEF12345');
  assert.equal(parseSerialResponse(withPrefix), 'F12301ABCDEF12345');

  const stripped63 = new Uint8Array(63);
  stripped63[0] = 1;
  stripped63[1] = 19;
  stripped63[2] = 2;
  writeAscii(stripped63, 3, 'F123Z3ASTROBOT123');
  assert.equal(parseSerialResponse(stripped63), 'F123Z3ASTROBOT123');

  // Short / malformed / empty serials
  assert.equal(
    parseSerialResponse(new Uint8Array([0x81, 1, 19, 2, 65, 66])),
    null,
  );
  assert.equal(
    parseSerialResponse(new Uint8Array([0x80, 1, 19, 2, ...new Array(20).fill(65)])),
    null,
  );
  assert.equal(
    parseSerialResponse(new Uint8Array([0x81, 2, 19, 2, ...new Array(20).fill(65)])),
    null,
  );
  assert.equal(
    parseSerialResponse(new Uint8Array([0x81, 1, 18, 2, ...new Array(20).fill(65)])),
    null,
  );
  assert.equal(
    parseSerialResponse(new Uint8Array([0x81, 1, 19, 3, ...new Array(20).fill(65)])),
    null,
  );

  const emptySerial = new Uint8Array(64);
  emptySerial[0] = REPORT_ID_SERIAL_RESPONSE;
  emptySerial[1] = 1;
  emptySerial[2] = 19;
  emptySerial[3] = 2;
  assert.equal(parseSerialResponse(emptySerial), null);
});

test('decodeDualSenseColor maps all 28 known colorway codes and rejects invalid inputs', () => {
  const cases: Array<[string, string | null]> = [
    ['F12300ABCDEF12345', 'White'],
    ['F12301ABCDEF12345', 'Midnight Black'],
    ['F12302ABCDEF12345', 'Cosmic Red'],
    ['F12303ABCDEF12345', 'Nova Pink'],
    ['F12304ABCDEF12345', 'Galactic Purple'],
    ['F12305ABCDEF12345', 'Starlight Blue'],
    ['F12306ABCDEF12345', 'Grey Camouflage'],
    ['F12307ABCDEF12345', 'Volcanic Red'],
    ['F12308ABCDEF12345', 'Sterling Silver'],
    ['F12309ABCDEF12345', 'Cobalt Blue'],
    ['F12310ABCDEF12345', 'Chroma Teal'],
    ['F12311ABCDEF12345', 'Chroma Indigo'],
    ['F12312ABCDEF12345', 'Chroma Pearl'],
    ['F12313ABCDEF12345', 'HyperPop Techno Red'],
    ['F12314ABCDEF12345', 'HyperPop Remix Green'],
    ['F12315ABCDEF12345', 'HyperPop Rhythm Blue'],
    ['F12330ABCDEF12345', '30th Anniversary'],
    ['F123Z1ABCDEF12345', 'God of War Ragnarok'],
    ['F123Z2ABCDEF12345', 'Spider-Man 2'],
    ['F123Z3ABCDEF12345', 'Astro Bot'],
    ['F123Z4ABCDEF12345', 'Fortnite'],
    ['F123Z6ABCDEF12345', 'The Last of Us'],
    ['F123ZAABCDEF12345', 'God of War 20th Anniversary'],
    ['F123ZBABCDEF12345', 'Icon Blue Limited Edition'],
    ['F123ZCABCDEF12345', 'Ghost of Yotei Limited Edition'],
    ['F123ZDABCDEF12345', 'Marathon Limited Edition'],
    ['F123ZEABCDEF12345', 'Genshin Impact Limited Edition'],
    ['F123ZFABCDEF12345', '007 First Light Limited Edition'],
    ['f123zfabcdef12345', '007 First Light Limited Edition'],
    ['F12399ABCDEF12345', null],
    ['F123', null],
    ['', null],
    ['F123é0ABCDEF', null],
    ['é12300ABCDEF', null],
  ];

  for (const [serial, expected] of cases) {
    assert.equal(decodeDualSenseColor(serial), expected, `serial=${serial}`);
  }
});
