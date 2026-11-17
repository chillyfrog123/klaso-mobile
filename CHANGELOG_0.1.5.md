# Klaso Mobile v0.1.5

- Android notifications for new messages, grades, substitutions and canceled lessons.
- Automatic checks on opening/resuming and every five minutes while the app is visible. Remembered accounts also check in the background about every 15 minutes, subject to Android scheduling.
- Profile → School notifications includes notification permission, automatic checking and a manual school-data check. Offline demo pauses school monitoring.
- Grades → New since last login filters newly detected grades and keeps their markers through refreshes. Semester filters combine with this view; averages still cover the complete semester.
- Room parsing now ignores blank lookup labels, supports numeric labels and nested lesson rooms, and preserves explicit room text when another classroom reference cannot be resolved.
- Per-account encrypted notification history, silent first-load baselines and stable grade IDs prevent repeated or spurious alerts.

Android version 0.1.5 / version code 6. ARM64, Android API 24 or later. This is a debug-signed development build. Actual notification delivery, battery restrictions and school-specific room responses require validation on a device with a live account.
