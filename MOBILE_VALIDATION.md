# Weekly grades and full room names — 20 November 2026

Local test APK: `mobile/artifacts/klaso-mobile-weekly-grades-rooms-debug.apk`, version 0.1.5 / version code 6. SHA-256: `008c110ed0278d0a23e12df8040ceb61f47d4dc3683b40422c083eded954f3d9`.

- Added the separate “Nowe w tym tygodniu” view, Monday through Sunday in Europe/Warsaw, based on the displayed grade date. The model does not contain a separate addition timestamp. Semester filtering, full-semester averages and the existing login-session badges retain their behavior.
- Room parsing preserves full inline names and scalar labels, resolves ID references to available full names before abbreviations, and retains symbol/number fallbacks. Timetable cards display the returned label without adding “Room”/“Sala”; details retain a separate field label.
- All 35 frontend unit tests and 32 browser scenarios passed, including week/year boundaries, Warsaw winter/summer Monday midnight, invalid dates, independent grade views, full averages and long classroom names at 320 px. Shared Rust tests passed: 27 mobile and 42 desktop, with one live-account test intentionally ignored in each suite.
- Production web build, Capacitor sync, ARM64 Rust rebuild, Android APK assembly and all 18 Android JVM tests passed. Android lint reported zero errors and four tool/resource warnings. APK signature and 16 KB ZIP alignment checks passed; the APK contains the weekly filter and empty-state translations.
- No live school account or physical-device validation was performed. Room parsing was checked with fixture responses; previously cached room labels need a timetable refresh.

# Text-selection menu — 20 November 2026

Local debug preview: `mobile/artifacts/klaso-mobile-text-selection-debug.apk`, version 0.1.5 / version code 6, 8,354,838 bytes. SHA-256: `cb64b2794db0d1f67eebaabd04dfe812a94b3971bf6ae511211184b44c45e9ee`. No release was published.

- Text selection uses an in-app trigger and the same themed `.choice` dialog as language/theme preferences. The native range remains adjustable until the user opens the actions. Copy and Share use the captured text; Select all stays within the message body. Back/Escape dismiss selection before navigating away. Editable fields retain their standard menus.
- The splash drawable now uses `android:windowBackground`, and the runtime theme clears the inherited general background that could draw the splash logo inside native popups.
- TypeScript/Vite production build and Capacitor sync passed. All 28 frontend unit tests and 29 Playwright scenarios passed. Eight selection regressions cover actual browser clipboard contents, scoped Select all, modal dismissal, native-event range refinement, Back/Escape, sharing, editable fields, and Polish dark 320 px layout. Native events and the external share sheet are simulated in browser tests.
- All 18 Android JVM tests, APK assembly and Android lint passed (zero errors, four tool/resource warnings). APK v2 signature verification and 16 KB ZIP alignment passed. The packaged CSS/JS filenames match the final web build. The existing ARM64 Rust library was reused; no Rust source changed.
- Dark/amber Polish and light English menu/trigger screenshots were visually reviewed at 320 and 390 px. Previews are bounded and actions remain accessible.
- No Android device was connected. Actual WebView long-press callbacks, handle dragging and native-toolbar suppression on the user's device, native clipboard/share actions, and installation remain unverified.

# School notifications and room parsing — 17 November 2026

Build: `mobile/artifacts/klaso-mobile-0.1.5-arm64-debug.apk`, version 0.1.5 / version code 6, 8,072,131 bytes. SHA-256: `4b428f344f0a00a01a999407a7921113a948b83a14b230a9ee648968e9e617ed`.

- TypeScript/Vite production build and Capacitor Android sync passed. All 28 frontend unit tests and 21 Playwright scenarios passed. New coverage checks grade identities, new-only/semester filtering with unchanged averages, automatic polling, disabled checks, hidden/resume behavior, demo pause, permission actions, account races and room text in cards/details. The new-grades screen was visually reviewed at 320 px.
- Mobile Rust: 26 passed, one live-account test intentionally ignored. Shared desktop compatibility: 41 passed, one live-account test ignored. Regression cases cover blank/number room labels, unresolved classroom references, nested room records and old grade snapshots without IDs. Formatting and Git whitespace checks passed.
- All 18 Android JVM tests passed, including 15 school-change tests for silent seeding, duplicate prevention, grade-ID migration, background/new-login checkpoints, per-account histories, message read-state independence, timetable changes, multi-page inbox checks, partial-failure recovery and combined alerts.
- ARM64 Rust backend rebuilt. App APK and instrumentation-test APK assembled. Android lint passed with zero errors and five existing tool/resource warnings. APK signature verification and 16 KB ZIP alignment passed; all native ELF LOAD segments use 16 KB alignment. Signing certificate matches the v0.1.4 APK; package ID, increasing version code, API 24 minimum and ARM64 ABI were verified.
- No device was connected and no live school account was used. Android notification permission/settings flows, WorkManager delivery under battery restrictions, notification taps, restart/account lifecycle and installation/upgrade require physical-device checks. The exact school-specific room response was not available; parser regressions and rendered room values passed fixture tests.

# Compact UI release validation — 17 November 2026

Release v0.1.4 uses Android version code 5. APK: `mobile/artifacts/klaso-mobile-0.1.4-arm64-debug.apk` (7,619,309 bytes).

- TypeScript/Vite production build, Capacitor Android asset sync, and the ARM64 Rust release build passed.
- 26 frontend unit tests and all 13 Playwright browser scenarios passed. These cover grades/details, translated preferences, timetable substitutions/cancellations, attendance filtering, inbox/notices, cached data/account boundaries and mocked updater flows.
- 160 layout checks passed across 320, 390, 700 and 1024 px widths, Android/iPhone styles, English/Polish and light/dark themes, plus tablet week-view checks. No page overflow or button/input height below 44 px was found. Selected narrow-phone and tablet screenshots were visually reviewed. Top/bottom safe-area checks passed for all five screens.
- 23 mobile Rust tests passed; the one real-account integration test remained ignored. Rust Clippy with warnings denied and formatting checks passed.
- All three Android/JUnit update-policy tests passed in a forced fresh run. Android lint passed with 0 errors and 5 existing warnings. The device-test APK compiled successfully.
- APK v2 signature verification, 16 KB ZIP alignment and ARM64 ELF load alignment checks passed. Package `pl.libruslite.mobile`, minimum API 24, target API 36, version name 0.1.4 and version code 5 were verified in the APK.
- The v0.1.4 signing certificate matches the published v0.1.3 APK; the older local APK hash was matched against its GitHub asset digest.
- Source formatting and Git whitespace checks passed. Tests used fictional fixtures, local HTTP fixtures or mocked native bridges; no real school account was used.
- No Android device was connected. Device instrumentation tests, live-account login, native installer permission flows and an actual in-place upgrade remain unverified.

APK SHA-256: `25fee220754629605c0d21793ec86ad9987a961d9c1ea6ae4133b9a50f95c4fe`.

# App updater validation — 17 November 2026

Release v0.1.3 includes the app updater. APK: `mobile/artifacts/klaso-mobile-0.1.3-arm64-debug.apk`, version 0.1.3 / version code 4.

- TypeScript/Vite build and Capacitor Android asset sync passed.
- 26 frontend unit tests passed, including 11 updater tests covering default opt-out, prompting/dismissal, persisted opt-in, manual checks and install, six-hour resume throttling, disabled checks, offline failures, installer permission denial, foreground deferral, unsupported platforms, and concurrent-operation suppression.
- All 13 Playwright browser scenarios passed, including five updater scenarios for prompts/manual installation, persisted automatic mode, disabled checks, permission-denial retry, Polish labels at 320 px, and native-only browser behavior. These updater scenarios mock the native bridge; they do not perform an Android installation. Screenshots were visually reviewed.
- Three Android/JUnit release-policy tests passed: numeric stable versions/downgrade rejection, ARM64 asset/digest validation, and rejection of foreign/insecure URLs.
- Android debug APK assembly and lint passed (0 errors, 5 warnings). The existing ARM64 Rust library was reused; no Rust source changed.
- The public GitHub latest-release endpoint returned v0.1.2 with an ARM64 APK and SHA-256 digest. No GitHub authentication or Librus account was used for this check.
- No Android device was attached. End-to-end native network checks, downloads, checksum/package/signature verification on-device, unknown-source permission round-trip, foreground handoff, installer cancellation, and a completed in-place upgrade still require physical-device testing. Use a same-key, higher-version test release; do not replace an official release solely for a test.

# Klaso Mobile 0.1.2 validation

Current release: see `CHANGELOG_0.1.2.md`. 23 Rust tests, 15 frontend unit tests and 8 browser scenarios passed. TypeScript/Vite, Android ARM64 build, Android lint, Rust Clippy, APK signature and 16 KB alignment checks passed. Physical-device behavior, live accounts and actual school room values have not been verified.

The following section records the earlier 0.1.0 baseline, not the 0.1.2 artifact checksum.

# Mobile validation — 13 November 2026

## Completed locally

- TypeScript type check and Vite production build passed; Capacitor Android assets synced.
- 15 frontend unit tests passed: grade bands, full-day attendance filtering, cache partitioning, stale-response guards, translation-key parity and platform design selection.
- 6 browser end-to-end scenarios passed (five shared-app scenarios plus the platform-style scenario): grades/details, persisted English/Polski and accents, timetable substitutions/cancellations/days off, attendance filter context, inbox reader, notices, 320 px layout, reduced motion, tablet split view, failed-refresh timestamp preservation, account-switch races, Material You Android and Liquid Glass iPhone preview.
- 22 Rust tests passed, including original authentication redirects/cookies, required-action detection, lazy renewal/retry, gateway pagination, messages decoding and grade calculations. The existing private-account integration test was intentionally not run.
- Rust Clippy with `-D warnings` and rustfmt checks passed.
- ARM64 Android Rust release library built successfully offline. JNI and portable C exports were verified; all ELF LOAD segments have 16 KB alignment (`0x4000`).
- Android SecureStore compiled against the installed Android SDK. This checks API/syntax compatibility, not device Keystore behavior.
- Phone/tablet screenshots reviewed. Browser previews are saved under ignored `mobile/artifacts/`, including `android-material-grades.png` and `iphone-glass-grades.png`.
- README.md unchanged. No real login credentials or private school data were used. No commits were created.

- Final Android build, app lint (0 errors; 6 warnings concerning tool versions and resource/configuration metadata), and app instrumentation-test APK assembly passed. Device tests were compiled but not executed.

## Android package

The native Java bridge and the Android APK build passed after dependency downloads resumed. Output: `mobile/artifacts/librus-lite-arm64-debug.apk`, 7,833,849 bytes (7.8 MB), package `pl.libruslite.mobile`, version 0.1.0, minimum API 24, target API 36, ARM64 only.

APK signature verification passed (v2 scheme), and `zipalign -c -P 16 4` passed. The debug signing key is local to the development machine and is not committed. The APK contains the Material You interface and compiled shared Rust library.

SHA-256: `BA7F9F2CC33F8189F639737E02B9B48B1A815FED58B507F876B471C2384252E6`

Rebuild with `mobile/build-android.ps1 -SkipInstall` and the SDK/JDK environment configured. Add `-Offline` for a cached-only build. SDK package auto-download is disabled in gradle.properties. No Android Studio or emulator image was downloaded.

## Native checks still needed

No device was attached when checked with adb. Install on an ARM64 Android device and run the included `NativeBoundaryTest` with `gradlew :app:connectedDebugAndroidTest`. It uses only fictional values and checks encrypted persistence, account-bound authentication of ciphertext, deletion and JNI demo calculations. No Android emulator image was downloaded.

Also check: cold startup offline, keyboard resizing, system Back, safe-area insets, screen-reader navigation, real Synergia login with and without remembering, restart/sign-out, expiry recovery, failed refresh and message read behavior. These live/device behaviors are implemented but have not been confirmed on a device or a real account. Do not run private-account tests without an appropriate user-provided testing session.

The iPhone visual design and portable Rust C boundary are present. A native Swift storage/Capacitor adapter, iOS binary and iOS device validation remain future platform work.
