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
