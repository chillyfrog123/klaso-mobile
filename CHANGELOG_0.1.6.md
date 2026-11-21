# Klaso Mobile v0.1.6

- Added a separate **New this week** grades filter for the current Monday–Sunday week in Europe/Warsaw. Existing semester averages and new-since-login markers remain independent.
- Preserved complete classroom names in timetable cards and details, including names resolved from room references.
- Improved timetable browsing with swipe navigation and clearer before/after cards for substitutions and lesson changes.
- Improved attendance date labels and updated the offline demo with examples for the new views.
- Refined the themed text-selection actions and native selection behavior.

Android version 0.1.6 / version code 7. ARM64, Android API 24 or later. This is a debug-signed development build, not a Play Store release. Keep the existing signing key for in-place updates. Physical-device and live-school-account validation were not performed for this release.

Validation: production web and ARM64 Android builds passed; 36 frontend unit tests, 33 browser tests, 28 Rust tests, Android unit tests and Android lint passed. The live-account Rust test was skipped. APK signature and 16 KB ZIP alignment verified; signing certificate matches v0.1.5.
