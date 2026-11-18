# Changelog (/docs/changelog)

All notable changes to this project will be documented in this file.
## [0.0.3] - 2026-11-18 [#003---2026-11-18]

### Bug Fixes [#bug-fixes]

- **viewmodel:** Ignore stale background catalog results when already loaded ([e81fc15](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/e81fc15dbc6f2191272a679e4f9e0ae20ab28386))

- **ui:** Fix StepIndicator and InlineMessage contrast under Fusion BasicTheme ([6f3b37c](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/6f3b37ce44ef2d3067283a88032578f80af682c2))

### Build & Packaging [#build--packaging]

- **appimage:** Bundle xdgdesktopportal platformtheme plugin ([ee1f05e](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/ee1f05e8f197c9c255e338ca361cd1d55602715f))

### Documentation [#documentation]

- Add feature-ideas.md with upcoming firmware updater ideas ([06d5719](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/06d5719d12ccb263524b5a1b811c9b5a05635159))

- Update AGENTS.md and README.md for safety and flow features ([11d1518](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/11d1518ff1c76e3463249a42771d14f88fde8b9e))

- Document --no-flash, native file picker, and new modules ([fc78f23](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/fc78f239038180b432bea9584c1b8707932375a9))

### Features [#features]

- **ui:** Implement safety validation, battery, verification, and flow updates ([4281dac](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/4281dac22a4af3ac06a7b378b1fa1aaaa2e6afe8))

- **no-flash:** Add NoFlashOutcome, NoFlashEnumerator, and simulated update ([1a32fec](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/1a32fecefc2f500786a6b0285319324737abe744))

- **viewmodel:** Expose no_flash_active on AppState and UiSnapshot ([593e4ba](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/593e4ba74bf41c5437d6e545153e070ae20e7370))

- **controller:** Wire --no-flash simulation into AppRuntime and tasks ([7a5a8cb](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/7a5a8cbae0e955ee742b78842a35e3307c1c50a7))

- **image:** Add local firmware validation against connected controller ([8567dfa](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/8567dfa83391b44e159b9c9ac9365452c13928ec))

- **transport:** Add HID bus type detection for USB vs Bluetooth ([2578a9d](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/2578a9d71b0a4656af63d157ca55bbf99c009164))

- **protocol:** Parse DualSense USB battery status from input report 0x01 ([6948ecd](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/6948ecd8647edcd2f0dd2339eb4b5fccc92a6a29))

- **updater:** Add bounded post-flash hardware verification ([d2d8c22](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/d2d8c2289913f8b24cbbd362dae1a37d2a038c87))

- **bridge:** Expose safety, battery, verification, and cache properties ([26c5f66](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/26c5f665994aa8deda45bfa833a1343168f67b39))

- **qml:** Split wizard steps and wire safety and flow UI ([8d41c93](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/8d41c93cdeb4a18576450ceba5bc2557c95c0bfa))

- **app:** Instantiate QApplication for native file dialogs ([3a6a0f3](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/3a6a0f3e5ea617ec0e50d72cbd0c23983da53c1a))

- **cli:** Add CLI argument parser for --no-flash and --help ([7b3427a](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/7b3427af9a21682bbe390fe469329c0bd1c3c984))

- **bridge:** Expose no_flash_active and startup no-flash configuration ([8787f49](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/8787f49c72917de402415b02ee023dff44cf7410))

- **app:** Wire CLI --no-flash parsing into main and QML indicator ([d1ca781](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/d1ca7812f47099629a22ff440ab8b9e12c34e4b4))

### Miscellaneous [#miscellaneous]

- Merge main into feat/safety-and-flow ([c8189ed](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/c8189edb56a222589812dcfbe46655e9b0c407d9))

### Refactor [#refactor]

- **packaging:** Update app ID to io.github.azorimor.linux_dualsense_fw_updater ([45b7b46](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/45b7b460ed269425ec1f573e736e94909e05a99c))

### Testing [#testing]

- **ui:** Update HidDeviceInfo fixtures for bus_type and add mock helpers ([b931955](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/b93195578031a55ebfc9ff5b98ffd7413f0ca25f))

- **ui:** Add unit and integration tests for Block 1 and 2 flows ([16caba3](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/16caba30d3d5b2ce8c573ffe54f71264285f2334))

- **ui:** Fix poll race in disconnect_monitor_test during flashing ([89006c5](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/89006c55c5db1db19b1f6909903750484ddd7cce))

- **ui:** Add unit and integration tests for --no-flash simulation ([937adef](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/937adef63dfdc5b1d55141d7211103686ff6caf4))

- **http:** Keep http_live_test strictly on local loopback servers ([85145f2](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/85145f27045e1bbb68fee6c365de7428805ac576))

- **app:** Add unit and integration tests for QApplication and portal theme ([c4ed98a](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/c4ed98a17684b1ee3ff7af81079f8bd50905b223))

## [0.0.2] - 2026-11-17 [#002---2026-11-17]

### CI / Workflows [#ci--workflows]

- **website:** Trigger website deployment when root CHANGELOG.md changes ([d4292b2](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/d4292b2d97d002a7686e402db9110f8070e85e8f))

- **release:** Fix Qt 6.8.3 modules and harden AppImage build checks ([025aff5](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/025aff53d7341cd077f946d0914b437220aa2742))

### Documentation [#documentation-1]

- **website:** Add changelog documentation page from root CHANGELOG.md ([f2e18f7](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/f2e18f7f659b2e56b924f19b6acc14245a170734))

### Features [#features-1]

- **website:** Strip redundant H1 from included MDX docs ([a6d57da](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/a6d57da6dc8c42cd79949a9d056bb5b1eacdf234))

### Testing [#testing-1]

- **website:** Add unit and build tests for changelog doc inclusion ([b640637](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/b640637b57429e321b677602e792c1726e8af0c5))

## [0.0.1] - 2026-11-17 [#001---2026-11-17]

### Bug Fixes [#bug-fixes-1]

- **ui:** Fall back to Fusion Quick Controls style when org.kde.desktop is missing ([97340c6](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/97340c63f9b220ca1f0c4d0fb4406788c7c8508a))

- **packaging:** Bundle Qt Quick, Wayland, and SVG plugins in AppImage ([011ea08](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/011ea083a6a53d1f278f8469497513e5a7729e5b))

- **release:** Enable git_only mode and subcrate tracking in release-plz ([3b02a1f](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/3b02a1f370d995d1b5cd7f0800b46c96a25578b3))

- **ci:** Select dualsense-updater tag from release-plz output ([f2e9b18](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/f2e9b183c8ac11c379a243c79c0f6157f4feae55))

- **release:** Release only on release PR merge and run release before release-pr ([617fb58](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/617fb587e1bdae92e409c2bd2670c1732fc84a07))

### CI / Workflows [#ci--workflows-1]

- **release:** Deploy website automatically after release assets upload ([67c1da1](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/67c1da13efd144ee518d8a253f3a31388f2cd3af))

- **release:** Bundle Qt 6.8, Kirigami, and Wayland in AppImage ([6d274f5](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/6d274f5d9c06564dae400c1cfbf36dd217e5b5cc))

### Documentation [#documentation-2]

- **ci:** Update CI workflow descriptions in README and AGENTS ([77326f8](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/77326f890e4db5d336bc339aac0d1faf7cdd668f))

- **licenses:** Add LGPL/BSD SPDX texts, notices, and in-app attribution ([23ffe4e](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/23ffe4e4becf4d2b17c0b8b6aff63029bd71d930))

### Features [#features-2]

- **website:** Rework download page for two builds and checksums ([1f0ceac](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/1f0ceac5648f6398a198a5edcb46453cb7375b5b))

- **packaging:** Bundle third-party licenses and library notices in AppImage ([9e994c2](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/9e994c2b1242f52aba867d1779c66ad274ca39a9))

### Miscellaneous [#miscellaneous-1]

- **reuse:** Cover generated CHANGELOG.md in REUSE.toml ([a734014](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/a73401491bd8a3aefb20edf7ab41f3c3eff12121))

- Revert changelog.md fix ([70d4ab2](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/70d4ab26728155ea57aada082c3716f3b843c86b))

### Performance [#performance]

- **ci:** Gate heavy check job on fast prechecks and run tests once ([89a11f8](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/89a11f8749b125b8a9f241687f890a6539ba241c))

## [0.0.0] - 2026-11-17 [#000---2026-11-17]

### Bug Fixes [#bug-fixes-2]

- **qml:** Wrap rootColumn in Item inside Kirigami.ScrollablePage ([854d292](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/854d2921f457892193f098f9ca68238b3ea6b053))

- **qml:** Polish step 0 and error action buttons ([32155a3](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/32155a3853738a81424f9c2fd1a073a26c033285))

- **build:** Suppress GCC 16 QChar SFINAE warning in cxx-qt build ([f64c28e](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/f64c28e66f9c8d9033511d56d5e0e33fa8f03b7e))

- **qml:** Close firmware file dialog when leaving firmware step ([aecaad9](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/aecaad9aa1e118b0e2f48708717f960e42399b99))

### Build & Packaging [#build--packaging-1]

- Convert to Cargo workspace and pin dependencies ([fd51f01](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/fd51f0147859ced0f977d6845f881bd998948e04))

- **workspace:** Add crates/dualsense-ui and pin tokio and ureq ([0824051](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/0824051d6bacc7dd065982820ec85591e5444807))

- **qt:** Add cxx-qt 0.10.0 and dualsense-ui deps with qmake6 build.rs ([03faf8e](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/03faf8e163cf29ac1b46692de035aab6009e5fd0))

- **linker:** Configure mold linker for Linux targets ([60e54c9](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/60e54c93abaf7cdf0b8b09a88d7b0d50c7793e19))

- **packaging:** Add Linux desktop entry, SVG icon, and AppImage script ([f3cd99c](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/f3cd99c8b04d104cc632144c1ea544907644812a))

- **website:** Configure Next.js static export, image optimizer, and Fumadocs MDX ([26a8ff9](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/26a8ff9efedf59c33945cded0f6fabb2aefe5ab5))

- **licenses:** Generate third-party license bundle in build.rs ([3bceb68](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/3bceb6840025cd604c2c65860bc9a09cdaf09598))

- **license:** Migrate workspace and license bundle to GPL-3.0-or-later ([343b9a5](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/343b9a5be6466fb7a568ff54eddeffff473c7d1c))

- **website:** Remove next-image-export-optimizer and image assets ([53a75c6](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/53a75c6af713a848edf9b61cb24cd49cefc1f580))

- **website:** Re-enable next-image-export-optimizer and gitignore cache ([1602fc4](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/1602fc44966be670fca3be38090234aa8e7e9eb2))

- **packaging:** Bundle hicolor icons and installer in tarball and AppDir ([0661bf2](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/0661bf21a37eeef8466a0cde0d12f9833caa0a22))

### CI / Workflows [#ci--workflows-2]

- **github:** Add manual Ubuntu 24.04 build, test, and coverage workflow ([2edf931](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/2edf931d5d40f76ad5b071e6fa2b7d89df9d174b))

- **github:** Update CI and add coverage, release, cache cleanup, and website workflows ([1899a36](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/1899a36deb5ca9b3d1639c274877e39a555b1dd0))

- **reuse:** Add SPDX headers to GitHub Actions workflow files ([93de70b](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/93de70b284be2ffed6fa1a0f0226f80df0dadfdd))

- **github:** Add libudev, libusb, GL, and xkbcommon dev packages on Ubuntu 24.04 ([cc4f681](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/cc4f68118fae2598518f9e453152c2dceed946ba))

- **license:** Add dedicated REUSE and cargo-deny license-check job ([86d9917](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/86d9917c670764690c62aa027570702079fd3040))

- **website:** Clarify image optimization step in deploy-website ([5cd8323](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/5cd8323bbe3986ff65619d8866a5efe757926e60))

- **workflows:** Upgrade GitHub Actions to Node 24 compatible versions ([624074d](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/624074dbaaf31074579219ac451bed2d86ab070e))

- **workflows:** Enable automatic PR, release, cache, and Pages triggers ([e8edfa5](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/e8edfa5a807427e5df0ff12ace50015c4ccbf546))

### Documentation [#documentation-3]

- **agents:** Document current architecture and git workflow rules ([89928d7](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/89928d7bfb9baf81722a7ff9e8bc0cb4c4888b38))

- **agents:** Require full logic tests and headless UI design ([a8d5402](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/a8d5402b8d369876d5b939048a4241b6c417190c))

- **readme:** Document KDE Kirigami GUI and Linux build prerequisites ([424bcd6](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/424bcd61d86d7c1758687e8dae4573ab908f3bf5))

- **agents:** Update architecture and verification docs for Kirigami workspace ([82f04e4](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/82f04e498c1b790a958fbbb06810db9a9cff28e3))

- **readme:** Add MIT license acknowledgments for original project ([a97cbf0](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/a97cbf0a8fb4ec9848aa7676d7a0ffbaad2e85aa))

- **agents:** Document src/tests, tests/, viewmodel split, and live USB polling ([745334e](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/745334e74dc45e3ae9f6bff8e42755e1c0b15ef5))

- **build:** Document mold linker and coverage script in README and AGENTS ([453baa1](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/453baa15ae0fc612d5e03ed26e0720eec696a663))

- **ci:** Document manual GitHub Actions workflow in AGENTS and README ([29cee6f](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/29cee6ff8c86c7a069475a0d73554d95150f3c91))

- **readme:** Rebrand and document AppImage, CI, and UI features ([4c83cba](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/4c83cba7add990516d6a1683d6dcde666e79a518))

- **website:** Add Fumadocs guides for setup, hardware, safety, and MIT ([9d31160](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/9d31160b667e9ed03b0ca0c3488d5a6167ab6151))

- **agents:** Sync file map and version 0.0.0 with new setup ([6efbff7](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/6efbff72b4591480fb49087998b1f8e91edba241))

- Update structural checks, README, and AGENTS for GPL-3.0-or-later and REUSE ([841e12f](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/841e12fe529add8a38f54274eaf6aeb5e5164b1e))

- Added software screenshots ([2c6d595](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/2c6d5958c877555116b48c77f4d6f51161cc55ad))

- **readme:** Cover assets in REUSE.toml and add GUI preview ([88e3a84](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/88e3a84533f64063ef7583b6a0ecacbbaddeeb5d))

- Document About/flash report UI, opt-in tests, and Node 24 workflows ([53c109c](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/53c109cffafcfe14f753200e0b51f9f500cf50e1))

- Add a logo and desktop icons ([8bc9636](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/8bc963629017a09b4af10c513030f1fbc69fdff1))

- Added logo and logo vector source file ([6f16d2f](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/6f16d2ff1c03af60ff2b50709dadca8cbd938eff))

- Document desktop icon installation and update AGENTS file map ([1eebf53](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/1eebf5384e987d933ed60e0b8521e5f838f2320e))

### Features [#features-3]

- **gui:** Add headless snapshot_adapter for UiSnapshot and file URL mapping ([6f0f658](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/6f0f658a1b2b781671d8d31c7bfa8847dbd68be8))

- **gui:** Implement cxx-qt DualSenseBridge for DefaultAppRuntime ([7c5d9d5](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/7c5d9d54fd8d5f63ffef23ab8347ee5a92e5b603))

- **qml:** Add KDE Kirigami guided firmware updater UI ([ea1ca9a](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/ea1ca9aece58e0f3b32f46c748ffa9535248a951))

- **gui:** Adapt board, color, serial, and target gating in snapshot_adapter ([9ec40d4](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/9ec40d40cd190c08ba7ed30f94f14fa4c42c7f34))

- **gui:** Expose board, color, and target gating in qt_bridge ([014b870](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/014b870d413530e8c18ae8f9c582d81205690163))

- **qml:** Show board info, warning, and target gating in Main.qml ([1c9dfa5](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/1c9dfa58907a09537c69e2254cf04b01069b9573))

- **website:** Add Fumadocs layouts, static search, and diagram ([e1bb2d1](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/e1bb2d1df3e07af43bc23dd27db547dcac072022))

- **licenses:** Add licenses module and unit tests ([bfb2220](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/bfb2220e1cd4f388c5c6da45a7084174df96843a))

- **qt-bridge:** Expose licensesText property on DualSenseBridge ([9319a6b](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/9319a6b4ad62915acbc4cb069aefe1b0966f2b19))

- **qml:** Add LicensesWindow and Help -> Licenses menu bar ([0f4602c](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/0f4602cebe9d4e80889f3a5d696d041c4a9855ad))

- **website:** Add build-time GitHub release fetcher and tests ([b068512](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/b068512fcde2891b45ba9a97dd945927f78e1a68))

- **website:** Add /download page with safety acknowledgment gate ([7da824f](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/7da824f27b6143b755d8e183d2c4d6cadee3e04a))

- **website:** Add screenshot assets and REUSE coverage ([bb0cc15](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/bb0cc150a4f8324d6c3920858a869a67444d3846))

- **website:** Render optimized workflow screenshots on home page ([fc4e5fb](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/fc4e5fb17bf83266c75387e3061c371de78f2f47))

- **bridge:** Expose app metadata and flash report properties ([2d34797](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/2d34797e7a0878aef25c40891b0f75267ba21352))

- **gui:** Add About dialog and flash report GitHub issue UI ([09c91d6](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/09c91d688bd25dd69f38490ae6f5de17df51c15a))

- **packaging:** Add XDG desktop installer script and complete desktop entry ([12f5f55](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/12f5f555e7a9a8ccbe5c4babcbc121a72902ed6a))

- **gui:** Embed desktop icons in Qt resources and set window icon ([22e2ed1](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/22e2ed12c982d513c45307d13832ba4b1c894a1d))

- **website:** Wire navbar logo, favicons, and web manifest with basePath ([a7cf446](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/a7cf4465aa9cada113f92489beda49c9a12b82f9))

### Miscellaneous [#miscellaneous-2]

- **git:** Add hooks enforcing branch protection and conventional commits ([8de382e](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/8de382eb781fc009159d97857b5465ae434d8a8d))

- **agents:** Add lifecycle hooks for branch and commit guardrails ([da5c5cb](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/da5c5cbd8889c53d4563c8ca871a3670d5e5afbf))

- **license:** Preserve nchie copyright alongside Azorimor in LICENSE ([f7f72ea](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/f7f72eaa6fb9356e6f1072c9003dcfda46f3f4af))

- Update README.md to include the repository link of the original author ([89ed6bb](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/89ed6bbce1b3f13b2bef95f6848c6c8a934fcbbe))

- **test:** Add workspace coverage script using cargo-llvm-cov ([5431e41](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/5431e41674d3e9d8c1ffe800d40f9f3c445433a9))

- **cargo:** Add deny.toml, rustfmt.toml, and clippy.toml ([8f7339f](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/8f7339fe864193180adfb50d34d85a259b4b4d35))

- **scripts:** Add structural rules verification script ([f4b6101](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/f4b610101c34a0eea293895df8fa14ca4c580650))

- **website:** Initialize package manifest and gitignore ([9443df4](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/9443df495941bce3940fb831932f6bdf0b13bb5d))

- **website:** Ignore image optimizer cache and tsbuildinfo ([28259b2](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/28259b284bb6b65d1bfb971102db53f7f391bfef))

- **deny:** Allow GPL-3.0-or-later licenses and fix deny.toml schema ([0d16068](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/0d160682f36c44c42522dbd1178c7df566b7c3af))

- **reuse:** Annotate root crate Rust files and tests ([03a463f](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/03a463f23bdf530c671c827b42ad76742ed3fe25))

- **reuse:** Annotate QML, config TOML, CLAUDE.md, and scripts ([46e11a1](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/46e11a1060b0d0ac24cc3fec950ade828dce1e50))

- **reuse:** Annotate website source and MDX documentation files ([fef099b](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/fef099ba5f240b4527f7f43720d6fa38abaf9c3c))

- **reuse:** Add REUSE.toml for non-commentable and config assets ([df103b6](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/df103b65566e997b3f00f34a376b8da5141404b9))

- **website:** Switch to black theme and enforce unofficial branding ([f6af662](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/f6af6623cf3ccb9dc8451f01e1dfb765f6de9b55))

- Add tmp folder to .gitignore ([56f2779](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/56f27797b0ab34889a56816ab2ebcaebd47b9d31))

- **github:** Add flash report issue template and REUSE annotation ([64801a0](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/64801a0c0ed1b8b5923821c7f498e79626875c7a))

- **packaging:** Move desktop icons into packaging/linux/icons ([c73c7eb](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/c73c7eb8f67a2895671e7414db6b6a868bd287bf))

- **website:** Move web brand assets from assets/web to website/public ([bd39001](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/bd390018dcc32feefc446200287d983111b6dfdb))

- **reuse:** Cover website/public/** in REUSE.toml and remove .license sidecars ([1545a6d](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/1545a6d397df7ce3c1228030d44ce904d989ed68))

- **license:** Restore copyright notices in MIT and GPL license files ([f4a49fe](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/f4a49fe63bf61236d76b13a0a92cab82057dab42))

### Refactor [#refactor-1]

- **gui:** Switch main.rs to Kirigami GUI and drop CLI modules ([366ebd3](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/366ebd3fa9353aaba1c83c006024fc3aed048422))

- **app:** Expose snapshot_adapter via lib.rs for integration tests ([5108e71](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/5108e71d50293da3b8864df1cc7342dbecb941ad))

### Testing [#testing-2]

- **app:** Extract snapshot_adapter unit tests into src/tests ([5504295](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/5504295c896e761348e334b2d6daa6b0b780b0b7))

- **app:** Add root snapshot and QML bindings integration tests ([c0d691d](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/c0d691d6577840f8c5f2a756e5694cae9b196dfa))

- **desktop:** Add desktop entry, icon, and packaging tests ([1e1aac1](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/1e1aac1570f5ff2566cef0bc1aa1ef60fe66d49b))

- **website:** Verify web brand assets, manifest, and basePath links ([2070be1](https://github.com/Azorimor/linux-dualsense-fw-updater/commit/2070be1bb86925f16ca56b86e9bcf45e2f0be9db))