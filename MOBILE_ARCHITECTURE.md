# Mobile architecture

## Framework and source reuse

React + TypeScript + Capacitor 8 provides a small shared mobile interface, native back-button support, safe-area/keyboard handling and an iOS-capable presentation layer. Android uses Material You / Material 3 styling with opaque tonal surfaces, rounded components and pill-shaped navigation indicators. The saved accent is the palette seed. iOS uses restrained Liquid Glass styling on navigation and toolbars, with opaque readable content cards. Native builds select their OS automatically; browser previews default to Android and accept `?platform=ios` for iPhone design review. No third-party authentication service or analytics is involved.

Based on chillyfrog123/librus-lite commit `93e2e437983d3566602c3f81b8a7edeac65af743`. `mobile/rust/src/lib.rs` source-links `src/api.rs`, `src/data.rs`, `src/i18n.rs` and `src/demo.rs`. The desktop backend is not reimplemented in JavaScript or Java. Shared changes include a translated category fallback and room normalization for symbols, IDs and scalar values. Authentication, endpoint conventions and grade calculations are unchanged.

The shared code still controls the state-bearing OAuth bootstrap, manual redirects and cookie jar, API-path session-cookie check, canonical TokenInfo endpoint, message-session handoff, server-advertised gateway pages, message page/limit parameters, numeric grade rules and averages. Summaries for all/first/second semester are computed by the original `grade_average`. Behavior grades never enter those summaries. Unknown weights keep weighted averages unavailable rather than guessing. The UI shows `[arithmetic]/[weighted]` and uses the written grade for its visual band.

## Android boundary

`LibrusPlugin.java` is a local Capacitor plugin. It executes every account operation, Rust request and encrypted write on one serial worker. Rust performs HTTPS requests through JNI; there is no localhost HTTP server, browser cookie store or JS fetch to Librus. Session credentials stay in the original zeroizing Rust credential type; framework/JNI strings may temporarily copy them. No credentials, tokens, cookies or message bodies are logged. Capacitor bridge logging is disabled.

The native session is restored lazily with `Session::deferred`. Startup reads the encrypted snapshot before automatic refresh. A data request can trigger renewal, and the original request is retried once by the original Rust API. School monitoring can request fresh data on startup/resume, on a foreground timer, or through Android WorkManager as described below. UI errors expose translated stable codes instead of raw server bodies/URLs. Required-action errors, including those wrapped by renewal context, link to the official website; CAPTCHA is never automated.

## Secure storage and cache consistency

`SecureStore.java` uses a 256-bit AES-GCM key generated in Android Keystore. Ciphertext contains a version byte, a fresh 12-byte nonce and an authenticated payload/tag. An SHA-256 account-derived filename is authenticated as associated data, preventing ciphertext from being substituted between account files. Files live in the app's no-backup directory; Android backup is disabled. AtomicFile commits avoid partially replacing valid snapshots.

Only non-sensitive language/accent preferences use localStorage. Passwords are persisted in encrypted storage only when **Keep me signed in** is checked. The active account marker is encrypted even when it contains just a login name. Sign-out removes that marker and remembered credentials, drops the session and clears visible state. Encrypted per-account datasets remain available after a later successful login to the same account; they are not exposed on the signed-out screen.

Cache keys separate datasets, timetable weeks, inbox pages and message details. A complete successful fetch plus encrypted write replaces a snapshot and its timestamp. Failed network requests, partial behavior-grade failures or persistence errors leave the previous complete snapshot and timestamp unchanged. Opening a message updates its cached read state without changing the collection refresh time. Startup shows cached data first; when the current timetable week is not cached, the latest cached week is shown with its actual dates.

The native queue serializes account switches with writes. The React RequestGate adds account-generation and per-request tickets, preventing old responses from changing the new account or replacing a newer request. Changing views also prevents old loading/errors from appearing on another tab.

## iOS boundary

The React UI and shared Rust logic are reusable. A C ABI (`librus_mobile_request` / `librus_mobile_free`, header in `mobile/rust`) is supplied for Swift integration. To ship on iOS, build the Rust static library for Apple targets, add the Capacitor iOS project, and implement the same bridge interface using Keychain-backed encryption and iOS file protection. Run native device and keyboard/accessibility tests on a Mac. An iOS binary and native Swift storage adapter are not included in this Android-first deliverable.

## School notifications and automatic refresh

Profile → School notifications controls automatic checks and notifications, enabled by default. Android notification permission is requested only through the Allow notifications button or when enabling the setting. Until permission is granted, school data still refreshes. The installed app checks grades, the inbox and the current/next timetable weeks on opening/resuming (coalescing returns within 30 seconds), and every five minutes while visible. Checks do not overlap. Hidden WebViews stop foreground polling. Demo mode pauses all school monitoring; sign-out stops it. The browser demo never contacts Librus.

Android WorkManager schedules connected-network background checks approximately every 15 minutes for the selected remembered account. Android may delay execution under battery/network restrictions; this is polling, so delivery is not immediate. Session-only sign-ins refresh in the foreground. All workers use the same serial account/session executor as the UI. Disabling monitoring cancels scheduled work and clears existing school notifications. Passwords, snapshots and tracking metadata remain in encrypted SecureStore files. No push broker or third-party school-data service is involved.

Each complete dataset is committed atomically with its notification history before posting an alert. Failed refreshes leave both intact. The first successful snapshot seeds history silently. Subsequent new grade IDs, message IDs and substantive upcoming substitution/cancellation changes produce generic category/count notifications. Message read/unread changes do not count as new messages. Notifications expose no subjects, grade values, senders or message bodies; their public lock-screen version only identifies new school information. Tapping an alert opens its section for the matching active account.

The encrypted grade checkpoint records the last successful foreground grade check. Opening a new app process, explicitly signing in or switching accounts freezes that checkpoint as the session's baseline. Background refreshes never advance it, so grades discovered while the app was closed remain new when it opens. Further refreshes keep the current session's markers. Grades → New since last login intersects those markers with the semester filter; all averages retain their full-semester meaning. Stable server grade IDs distinguish otherwise identical entries. Older snapshots without IDs migrate through the grade fields and duplicate occurrence index to avoid treating every old grade as new after updating.

## Android app updates

Profile → App updates provides a manual check, automatic checks (on by default), and automatic updates (**off by default**). These non-sensitive preferences use the `app-updates-v1` localStorage key. Disabling automatic checks also disables automatic updates; opting into automatic updates enables checks. The updater checks on launch and subsequent app resumes, at most once per six hours within a running session. Manual checks bypass that interval. There is no background service or polling while the app is closed.

`UpdateController` owns update state independently of school accounts. A newer version prompts with release notes and Download and update / Later. Later suppresses repeated automatic prompts for that version during the session; a manual check can show it again. Automatic mode downloads and opens the Android installer once per version per session, with manual retry available after cancellation or failure. It waits for the app to return to the foreground before starting a download or opening the installer. Android still requires installation confirmation and, on Android 8+, permission to install from this app. Granting permission resumes the prepared update; denying it leaves a retryable message. A process restart during the permission flow requires retrying the update.

`AppUpdaterPlugin.java` runs on a separate worker from Librus requests and uses the public `https://api.github.com/repos/chillyfrog123/klaso-mobile/releases/latest` endpoint without credentials. Checks and downloads may use mobile data. Browser/iOS previews do not check or download updates. Only stable `vMAJOR.MINOR.PATCH` / `MAJOR.MINOR.PATCH` releases newer than the installed app qualify. The release must contain an uploaded `klaso-mobile-VERSION-arm64[-debug|-release].apk` asset (the bracketed suffix is optional), a GitHub `sha256:` digest, and a size between 1 byte and 150 MiB. Release tags must match the APK version; future releases must increase Android `versionCode` as well as `versionName` and retain the signing key. The existing public v0.1.2 release uses the supported naming/digest format.

Before installing, the plugin fetches release metadata again, follows only HTTPS GitHub/release-asset redirects, and verifies the downloaded size, SHA-256, package ID, version name, increasing version code, minimum Android API, and signing certificates against the installed app. It checks the prepared file again after the permission flow. Partial downloads stay in private cache and are removed after failure. Only the update cache subdirectory is shared through FileProvider. A verified complete download can be reused on retry. Signing-key rotation is deliberately not accepted by this updater; migration to a different signing key needs a separate distribution plan. Android's installer performs the final platform installation checks.

## Deliberate behavior

- Inbox search applies to the current page; this is explicitly labeled. Synergia gateway collection requests do not invent pagination parameters.
- Attachment names/availability are shown, with an official-site action for download.
- Empty timetable days get a red day-off panel; the source desktop timetable model does not expose a separate holiday label.
- Unknown attendance types use an explicit Other filter instead of silently classifying them as Late.
- Demo data is generated from the original Rust fixtures. It is fictional, memory-only, clearly labeled and does not trigger Librus requests. Entering/leaving demo preserves the existing live account's UI snapshot separately.





Version 0.1.2 changes and validation are documented in `CHANGELOG_0.1.2.md`. The repository is https://github.com/chillyfrog123/klaso-mobile. Saved accounts are kept in the encrypted `accounts-v1` registry; the active marker stores only the selected owner. Switching restores that account lazily and displays its encrypted cache.
