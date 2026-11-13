# Klaso Mobile 0.1.2

- Renamed the mobile app to Klaso Mobile, with repository and GitHub actions under Profile → About the app.
- Replaced every native select control with an accessible in-app choice dialog. Language, semester and theme selectors no longer open Android's themed popup.
- Added saved System / Light / Dark appearance settings.
- Attendance entries run left to right in lesson-number order inside each day. Day filtering retains all entries for context.
- Full sixes are blue and retain the reduced-motion-aware light sweep. Behavior appears above academic summaries.
- Added encrypted storage of multiple remembered accounts, quick switching, removal of saved credentials and separate per-account snapshots. Existing remembered credentials migrate on startup. Sign-out forgets the current account; switching keeps saved accounts.
- Unread messages use a stronger background, bold title and accent edge; read messages are quieter.
- Timetable normalization now handles embedded room symbols, lookup IDs and scalar room values.

The Android application ID remains unchanged to preserve encrypted data when upgrading the prior development APK. Display name and version are Klaso Mobile 0.1.2 (versionCode 3).

Verification: frontend typecheck/build, 15 unit tests, 8 browser scenarios, 23 Rust tests, and Rust Clippy passed. One private-account test remains intentionally ignored. Android APK build passed. No real account credentials or school data were used in automated checks. Physical-device login, actual school room payloads and switching real accounts still require device validation.
