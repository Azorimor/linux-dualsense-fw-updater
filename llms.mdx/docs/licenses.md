# Licenses & Acknowledgments (/docs/licenses)

## Upstream Attribution [#upstream-attribution]

The core DualSense USB HID protocol implementation, firmware image validation, and update state machine logic in `crates/dualsense-core` are built on top of **[nchie&apos;s original CLI project](https://github.com/nchie/dualsense-updater-rs)** under the MIT License and remain licensed under **`MIT AND GPL-3.0-or-later`**:

```text
Copyright (c) 2026 nchie
Copyright (c) 2026 Azorimor
```

## Full Licenses & Third-Party Notices [#full-licenses--third-party-notices]

To keep licensing information in a single canonical place, all full license texts, third-party notices, and dependency metadata are maintained directly in the GitHub repository:

- **[`LICENSES/` directory](https://github.com/Azorimor/linux-dualsense-fw-updater/tree/main/LICENSES)** &mdash; Full SPDX license texts (`GPL-3.0-or-later`, `MIT`, `Apache-2.0`, `LGPL-*`, `BSD-3-Clause`).
- **[`packaging/linux/THIRD_PARTY_NOTICES.txt`](https://github.com/Azorimor/linux-dualsense-fw-updater/blob/main/packaging/linux/THIRD_PARTY_NOTICES.txt)** &mdash; Third-party library notices and Corresponding Source availability.
- **[`REUSE.toml`](https://github.com/Azorimor/linux-dualsense-fw-updater/blob/main/REUSE.toml)** &mdash; FSFE REUSE copyright and license annotations.
- **[`deny.toml`](https://github.com/Azorimor/linux-dualsense-fw-updater/blob/main/deny.toml)** &amp; **[`Cargo.lock`](https://github.com/Azorimor/linux-dualsense-fw-updater/blob/main/Cargo.lock)** &mdash; Rust dependency license policy and locked crate list.

Complete per-crate license notices are also bundled inside the desktop application under **Help &rarr; Licenses**.