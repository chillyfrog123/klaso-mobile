# Klaso Mobile

A lightweight, unofficial mobile companion for Librus Synergia. Android first, with Material You styling and a React/Capacitor interface backed by Rust code adapted from [Librus Lite](https://github.com/chillyfrog123/librus-lite).

## Download

Get the **Android ARM64 APK** from [GitHub Releases](https://github.com/chillyfrog123/klaso-mobile/releases). Version **0.1.2** requires Android 7.0 / API 24 or later. The current APK is a debug-signed development build.

Copy the APK to your phone and open it to install. Try **Profile → Explore offline demo** for fictional data, or sign in with your Synergia account.

## Features

- Timetable with lesson details, substitutions and cancellations.
- Academic grades and averages, with separate behavior grades.
- Attendance grouped by day, with horizontal lesson entries and filters.
- Paginated inbox, message reader and school notices.
- Multiple encrypted remembered accounts and separate cached datasets.
- English / Polski, System / Light / Dark themes and four accent colors.
- Material You on Android; Liquid Glass in the iPhone preview.

A native iOS build is not yet available. The Swift storage/bridge adapter and device validation remain future work.

## Privacy

No analytics or telemetry. Credentials go directly to Librus. Remembered credentials and snapshots use Android Keystore-backed encryption. Failed refreshes preserve cached data and its last successful refresh time.

CAPTCHA and additional verification must be completed on the official Librus website. Opening a message may mark it read remotely. School content stays in its original language.

## Development

The mobile project is in `mobile/`. It reuses the Rust authentication, API models and calculations in `src/`. Original desktop sources remain for backend reuse; the primary deliverable here is the mobile app.

Browser preview with fictional data:

```sh
cd mobile
npm ci
npm run dev
```

Add `?platform=ios` to preview the iPhone appearance. Live sign-in uses the Android native bridge.

- [Android setup, builds and installation](MOBILE_SETUP.md)
- [Architecture and encrypted storage](MOBILE_ARCHITECTURE.md)
- [Validation and remaining device checks](MOBILE_VALIDATION.md)
- [Version 0.1.2 changes](CHANGELOG_0.1.2.md)

Live login, school-specific room values and device behavior still require physical-device validation. Never commit credentials, private school data or machine-local build configuration.

Klaso Mobile is independent and unofficial, and is not affiliated with Librus.
