# Supported Hardware & Board Revisions (/docs/supported-hardware)

The updater inspects Feature Report `0x20` (`hw_info` and `fw_series`) along with the USB Product ID to determine your controller&apos;s mainboard revision and matching Sony firmware target.

## USB Hardware Identifiers [#usb-hardware-identifiers]

| Device | USB Vendor ID | USB Product ID | Testing Status |
| :--- | :--- | :--- | :--- |
| Sony DualSense | `0x054c` | `0x0ce6` | Tested on standard DualSense |
| Sony DualSense Edge | `0x054c` | `0x0df2` | **Untested** (flagged before flashing) |

## Board Revision to Sony Target Mapping [#board-revision-to-sony-target-mapping]

Board revision is decoded from `(hw_info >> 8) & 0xff` in Feature Report `0x20`:

| Hardware Byte | Board Revision | Sony Target ID | Official Catalog Support |
| :--- | :--- | :--- | :--- |
| `0x03` | `BDM-010` | &mdash; | **Unsupported** (not in Sony&apos;s PC update catalog) |
| `0x04` | `BDM-020` | `0004` | Supported |
| `0x05` | `BDM-030` | `0004` | Supported |
| `0x06` | `BDM-040` | `0004` | Supported |
| `0x07` / `0x08` | `BDM-050` | `000B` | Supported |
| `0x09` | `BDM-060R` | `000E` | Supported |
| `0x11` | `BDM-060M` | `000E` | Supported |
| `0x13` | `BDM-060X` | `000E` | Supported |
| PID `0x0df2` | `DualSense Edge` | `0044` | Supported in catalog (**Untested**) |

> [!WARNING] BDM-010 Launch Controllers
>
> Launch-era `BDM-010` DualSense controllers are not supported by Sony&apos;s official PC firmware catalog (`info.json`). The application detects `BDM-010` and disables official catalog flashing for safety.

## Sony Firmware Sources [#sony-firmware-sources]

Version metadata is fetched from:

```text
https://fwupdater.dl.playstation.net/fwupdater/info.json
```

Firmware binary images are downloaded from:

```text
https://fwupdater.dl.playstation.net/fwupdater/fwupdate<target>/<version>/FWUPDATE<target>.bin
```

For example, version `0x0630` for a `BDM-050` controller (`000B`) resolves to:

```text
https://fwupdater.dl.playstation.net/fwupdater/fwupdate000B/0x0630/FWUPDATE000B.bin
```

## Serial Number & Colorway Detection [#serial-number--colorway-detection]

When the controller responds to Feature Report `0x80` / `0x81`, the updater reads the controller serial number (when permissions allow) and performs best-effort colorway identification from serial characters `[4..6]` (covering standard finishes such as White, Midnight Black, Cosmic Red, Nova Pink, Galactic Purple, Starlight Blue, Chroma variants, and limited editions).