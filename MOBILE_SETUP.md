# Klaso Mobile — Android setup

## Install the built app

**Built and verified:** `mobile/artifacts/klaso-mobile-0.1.2-arm64-debug.apk` (7.8 MB), version 0.1.2, ARM64, Android 7 / API 24 or later. APK v2 signature and 16 KB ZIP alignment checks passed. Copy it to an ARM64 Android phone (Android 7 / API 24 or later), open it and allow installation from your file manager when Android asks. This is a debug-signed development build, not a Play Store release. No account or school data is bundled.

Alternatively, with USB debugging enabled:

```sh
adb install -r mobile/artifacts/klaso-mobile-0.1.2-arm64-debug.apk
```

Open **Profile → Explore offline demo** to explore fictional data without contacting Librus. In the installed Android app, Profile also supports real Synergia login. Opening a real message can mark it read remotely. Use **Profile → Open official Librus website** if Librus asks for CAPTCHA, account setup, a password change or additional verification. Complete those steps yourself, then sign in again.

Android uses Material You styling; iPhone uses Liquid Glass styling. The browser preview defaults to Android; add `?platform=ios` to preview the iPhone appearance. An iOS native build is not included yet.

English is the default. Profile saves English / Polski and Cobalt / Emerald / Violet / Amber. School-provided text is not translated. The green grade highlight respects the system reduced-motion preference.

## Build from source

Prerequisites:

- Node.js 22 or later and npm.
- Stable Rust, plus `rustup target add aarch64-linux-android`.
- `cargo install cargo-ndk --locked` (tested with cargo-ndk 4.1.2).
- JDK 21 or later (tested with JDK 24).
- Android command-line tools, SDK platform 36, build-tools 36.0.0, and NDK 28.2.13676358. Android Studio is optional.
- `JAVA_HOME` points to the JDK; `ANDROID_HOME` points to the SDK. `ANDROID_NDK_HOME` may override the NDK location.

Install the SDK packages with the official SDK manager:

```sh
sdkmanager --licenses
sdkmanager "platforms;android-36" "build-tools;36.0.0" "platform-tools" "ndk;28.2.13676358"
```

From the repository root, in PowerShell:

```powershell
./mobile/build-android.ps1
```

For an already-installed npm dependency tree, use `-SkipInstall`. To prohibit all build downloads, use `-SkipInstall -Offline`; it fails explicitly if required dependencies are missing. The script builds the web interface, syncs Capacitor, compiles the original Rust API code for ARM64, then packages the APK. Machine-local SDK paths are read from environment variables and never written into tracked files. Generated native libraries, outputs, private signing keys and local.properties are ignored by Git.

Equivalent commands on macOS/Linux (with the same prerequisites):

```sh
cd mobile
npm ci
npm run android:sync
cd rust
cargo ndk -t arm64-v8a --platform 24 -o ../android/app/src/main/jniLibs build --release --locked
cd ../android
./gradlew assembleDebug
```

The Gradle build refuses to package the app if the Rust library is missing. Only ARM64 is packaged to keep the download small. An x86 emulator build requires adding the Rust x86_64 Android target, building that ABI, and changing `abiFilters`; it is not part of the delivered APK.

## Local preview and checks

```sh
cd mobile
npm ci
npm run dev
npm test
npm run build
npm run test:e2e
cargo test --manifest-path rust/Cargo.toml
```

Browser preview supports the offline demo only; it cannot send login credentials. Playwright requires its Chromium browser (`npx playwright install chromium`), or set `PLAYWRIGHT_EXECUTABLE_PATH` to an already-installed compatible Chromium executable. Browser tests use fictional fixtures and mock failures, never a real account.

`README.md` is intentionally untouched. See `MOBILE_ARCHITECTURE.md` for native storage, session recovery, cache behavior and iOS boundaries. See `MOBILE_VALIDATION.md` for the checks actually completed and remaining device validation.





Version 0.1.2 changes and validation are documented in `CHANGELOG_0.1.2.md`. The repository is https://github.com/chillyfrog123/klaso-mobile. Saved accounts are kept in the encrypted `accounts-v1` registry; the active marker stores only the selected owner. Switching restores that account lazily and displays its encrypted cache.
