# Mobile architecture

## Framework and source reuse

React + TypeScript + Capacitor 8 provides a small shared mobile interface, native back-button support, safe-area/keyboard handling and an iOS-capable presentation layer. Android uses Material You / Material 3 styling with opaque tonal surfaces, rounded components and pill-shaped navigation indicators. The saved accent is the palette seed. iOS uses restrained Liquid Glass styling on navigation and toolbars, with opaque readable content cards. Native builds select their OS automatically; browser previews default to Android and accept `?platform=ios` for iPhone design review. No third-party authentication service or analytics is involved.

Based on chillyfrog123/librus-lite commit `93e2e437983d3566602c3f81b8a7edeac65af743`. `mobile/rust/src/lib.rs` source-links `src/api.rs`, `src/data.rs`, `src/i18n.rs` and `src/demo.rs`. The desktop backend is not reimplemented in JavaScript or Java. Shared changes include a translated category fallback and room normalization for symbols, IDs and scalar values. Authentication, endpoint conventions and grade calculations are unchanged.

The shared code still controls the state-bearing OAuth bootstrap, manual redirects and cookie jar, API-path session-cookie check, canonical TokenInfo endpoint, message-session handoff, server-advertised gateway pages, message page/limit parameters, numeric grade rules and averages. Summaries for all/first/second semester are computed by the original `grade_average`. Behavior grades never enter those summaries. Unknown weights keep weighted averages unavailable rather than guessing. The UI shows `[arithmetic]/[weighted]` and uses the written grade for its visual band.

## Android boundary

`LibrusPlugin.java` is a local Capacitor plugin. It executes every account operation, Rust request and encrypted write on one serial worker. Rust performs HTTPS requests through JNI; there is no localhost HTTP server, browser cookie store or JS fetch to Librus. Session credentials stay in the original zeroizing Rust credential type; framework/JNI strings may temporarily copy them. No credentials, tokens, cookies or message bodies are logged. Capacitor bridge logging is disabled.

The native session is restored lazily with `Session::deferred`. Startup reads the encrypted snapshot without a network login. A data request can trigger renewal, and the original request is retried once by the original Rust API. There is no timer, polling service or background keep-alive. UI errors expose translated stable codes instead of raw server bodies/URLs. Required-action errors, including those wrapped by renewal context, link to the official website; CAPTCHA is never automated.

## Secure storage and cache consistency

`SecureStore.java` uses a 256-bit AES-GCM key generated in Android Keystore. Ciphertext contains a version byte, a fresh 12-byte nonce and an authenticated payload/tag. An SHA-256 account-derived filename is authenticated as associated data, preventing ciphertext from being substituted between account files. Files live in the app's no-backup directory; Android backup is disabled. AtomicFile commits avoid partially replacing valid snapshots.

Only non-sensitive language/accent preferences use localStorage. Passwords are persisted in encrypted storage only when **Keep me signed in** is checked. The active account marker is encrypted even when it contains just a login name. Sign-out removes that marker and remembered credentials, drops the session and clears visible state. Encrypted per-account datasets remain available after a later successful login to the same account; they are not exposed on the signed-out screen.

Cache keys separate datasets, timetable weeks, inbox pages and message details. A complete successful fetch plus encrypted write replaces a snapshot and its timestamp. Failed network requests, partial behavior-grade failures or persistence errors leave the previous complete snapshot and timestamp unchanged. Opening a message updates its cached read state without changing the collection refresh time. Startup shows cached data first; when the current timetable week is not cached, the latest cached week is shown with its actual dates.

The native queue serializes account switches with writes. The React RequestGate adds account-generation and per-request tickets, preventing old responses from changing the new account or replacing a newer request. Changing views also prevents old loading/errors from appearing on another tab.

## iOS boundary

The React UI and shared Rust logic are reusable. A C ABI (`librus_mobile_request` / `librus_mobile_free`, header in `mobile/rust`) is supplied for Swift integration. To ship on iOS, build the Rust static library for Apple targets, add the Capacitor iOS project, and implement the same bridge interface using Keychain-backed encryption and iOS file protection. Run native device and keyboard/accessibility tests on a Mac. An iOS binary and native Swift storage adapter are not included in this Android-first deliverable.

## Deliberate behavior

- Inbox search applies to the current page; this is explicitly labeled. Synergia gateway collection requests do not invent pagination parameters.
- Attachment names/availability are shown, with an official-site action for download.
- Empty timetable days get a red day-off panel; the source desktop timetable model does not expose a separate holiday label.
- Unknown attendance types use an explicit Other filter instead of silently classifying them as Late.
- Demo data is generated from the original Rust fixtures. It is fictional, memory-only, clearly labeled and does not trigger Librus requests. Entering/leaving demo preserves the existing live account's UI snapshot separately.





Version 0.1.2 changes and validation are documented in `CHANGELOG_0.1.2.md`. The repository is https://github.com/chillyfrog123/klaso-mobile. Saved accounts are kept in the encrypted `accounts-v1` registry; the active marker stores only the selected owner. Switching restores that account lazily and displays its encrypted cache.
