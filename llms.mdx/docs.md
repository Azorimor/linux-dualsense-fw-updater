# Overview (/docs)

**Unofficial Linux DualSense Firmware Updater** (`Azorimor/linux-dualsense-fw-updater`) is an unofficial, community-maintained Linux desktop utility built with Rust and KDE Kirigami (Qt 6) for inspecting DualSense controller hardware and flashing firmware updates over a wired USB connection. It has **no affiliation, endorsement, or relation to Sony Interactive Entertainment or PlayStation**.

> [!WARNING] Unofficial Project & Safety Disclaimer
>
> This is an unofficial, community-maintained project with no affiliation, endorsement, or relation to Sony Interactive Entertainment or PlayStation. Only use this software if you have no access to Sony&apos;s official PlayStation Accessories / DualSense firmware updater for Windows or a PlayStation 5 console. Firmware upgrades are auto-committed by the controller even if verify and finalize steps do not run, and flashing firmware can permanently damage or brick your controller. Use at your own risk.

## Guided 4-Step Workflow [#guided-4-step-workflow]

The desktop GUI walks you through four stages while monitoring USB connection state:

1. **Controller** &mdash; Scans wired USB HID devices (`054c:0ce6` DualSense, `054c:0df2` DualSense Edge) and reads Feature Reports `0x20` and `0x81` to display current firmware version, build date, board revision (`BDM-*`), firmware series, and controller colorway.
2. **Firmware** &mdash; Either fetches Sony&apos;s official firmware catalog (`info.json`) for your detected hardware target (`0004`, `000B`, `000E`, or `0044`) or validates a local `.bin` image from disk.
3. **Review** &mdash; Compares current and target versions, warns if the controller is already up to date or if the target is untested (`DualSense Edge`), and requires explicit confirmation that firmware updates cannot be reversed.
4. **Update** &mdash; Streams downloads with safe pre-flash cancellation, writes firmware in 32 KiB blocks via Feature Report `0xF4`, polls status via `0xF5`, verifies, and finalizes.

## Key Features [#key-features]

- **Automatic Board & Target Resolution**: Decodes hardware revision (`BDM-020` through `BDM-060R/M/X`) and firmware series to match the right Sony update target automatically.
- **Live USB Disconnect Monitoring**: Polls USB device presence every 800 ms outside active flashing so unplugging a controller immediately resets to scan safely.
- **Plain-English Error Recovery**: Translates low-level USB HID, catalog, and image validation failures into actionable recovery steps.
- **Built-in Licenses Dialog**: Includes an in-app Licenses & Acknowledgments viewer covering GPL-3.0-or-later, MIT (nchie), and third-party Rust/Qt/KDE credits.