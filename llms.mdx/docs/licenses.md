# Licenses & Acknowledgments (/docs/licenses)

## Project License [#project-license]

**Unofficial Linux DualSense Firmware Updater** is licensed under the **GNU General Public License v3.0 or later (`GPL-3.0-or-later`)** and follows the [FSFE REUSE](https://reuse.software/) specification. Full license texts live in `LICENSES/GPL-3.0-or-later.txt` and `LICENSES/MIT.txt`.

```text
Copyright (C) 2026 Azorimor <azorimor@gmail.com>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
```

The core DualSense USB HID protocol implementation, firmware image validation, and update state machine logic in `dualsense-core` are adapted from [nchie&apos;s original CLI project](https://github.com/nchie/dualsense-updater-rs) and remain licensed under **`MIT AND GPL-3.0-or-later`**:

```text
Copyright (c) 2026 nchie
Copyright (c) 2026 Azorimor
```

## Acknowledgments [#acknowledgments]

- **[nchie](https://github.com/nchie/dualsense-updater-rs)** &mdash; The core DualSense USB HID protocol implementation, firmware image validation, and update state machine logic in `dualsense-core` are adapted from nchie&apos;s original CLI project under the MIT License.
- **Open-Source Ecosystem** &mdash; Built with Rust, `hidapi`, `tokio`, `ureq`, `cxx-qt`, Qt 6, KDE Frameworks 6 Kirigami, and documented with Next.js and Fumadocs. Full dependency license notices are also accessible directly inside the desktop application via the built-in Licenses dialog.