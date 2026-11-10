# Librus Lite

A small Windows desktop starter written in Rust, with native egui rendering (no browser runtime or C++ layer).

- Polish native interface with a calm neutral background, subtle glass surfaces, compact summary strips, clearer typography and four saved accent colors (cobalt, emerald, violet and amber). Message content has a wider reading pane, and timetable cards use tighter spacing to show more lessons.
- Grade badges for `5+` and `6` have a soft diagonal light sweep, lasting 1.2 seconds every six seconds. It stays behind the text and inside the rounded badge; repainting is limited to visible badges in the focused grades view, with a single scheduled wake-up between sweeps.
- Account tab: Synergia login, status and sign out.
- Keep me logged in: stores username/password in Windows Credential Manager. Startup restores cached data and prepares a session without a network request; authentication happens on the next data call. No plaintext credential file. Unchecking the option or signing out removes the saved entry.
- Grades: grouped by subject, semester filter, search, and individual grade details. Overall and per-subject averages display `[arithmetic]/[weighted]` values for all numeric constituent grades, using category weights and the school's plus/minus settings. Non-numeric, proposed/final/semester and behaviour grades are excluded. These are local requested averages even when a category's official average display is disabled. An unset category weight defaults to 1; unavailable category metadata produces a dash for the weighted value. Formal behaviour grades, plus a subject named Zachowanie when present, stay in the fixed toolbar on the right.
- Attendance: daily groups, O/N/NU/Z badges, subject tooltips and a multi-select day filter. The displayed percentage uses Librus's `IsPresenceKind` classification and excludes unknown types; it is not a separately supplied official average.
- Messages: full-height inbox and reading panes with independent scrollbars, a stronger accent background for the inbox, and a colored outline/stripe for the selected message. Paginated inbox, sender/topic search on the current page, decoded full content, and attachment names. Opening a message may mark it read in Librus. Downloading attachments is available through the official website; sending messages is not implemented.
- School notices: searchable titles/content and expandable details. Reading a notice here does not mark it read on the server.
- Timetable: previous/current/next week, parallel lessons, substitutions and canceled classes. Dates use Europe/Warsaw. The full week fits the available viewport, with aligned lesson slots, orange substitution tiles and red canceled tiles. Empty days have full-height red panels. Hover a tile for the complete lesson details.
- Last successful datasets, original update dates and opened message content are saved per account under `%LOCALAPPDATA%/LibrusLite/cache/`, encrypted with Windows DPAPI. Cache files never include passwords. Data is written to a temporary file, synced and atomically replaced; request errors do not modify snapshots or timestamps, and failed disk replacements preserve the previous file. Each tab retains its last successful context (timetable week or inbox page). A saved account selector permits offline viewing even without remembered credentials. Signing out removes the selector and credentials; the encrypted account snapshot remains available on a later login to that account.
- The selected color is stored as a local JSON setting under `%LOCALAPPDATA%/LibrusLite/appearance.json`.
- HTTP requests run off the UI thread, use TLS, timeouts and a shared cookie jar. Login redirects are restricted to the two expected Librus hosts; message initialization also permits `wiadomosci.librus.pl`. Late responses from a previous account or previous timetable request are discarded.
- Synergia gateway collections are requested without invented `page`/`limit` parameters. Further pages are fetched only from a `Pages.Next` link supplied by the server, restricted to the same origin and collection path. The separate messages API retains its documented page/limit parameters.
- The known messages handoff is upgraded from HTTP to HTTPS before any request is sent; cookies and handoff query parameters are never sent over the advertised plaintext URL.

## Build and run

Install stable Rust and either Visual Studio C++ Build Tools (MSVC target), or MinGW-w64 (GNU target). Then:

```powershell
cargo run --release
```

Executable: `target/release/librus-lite.exe`. `cargo test --release` checks login redirects/cookies, required-action screens, Polish text decoding, ISO weeks and timetable normalization. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` check code quality.

You can also run `./run.ps1` in PowerShell; it builds the optimized executable and starts the app.

`target/release/librus-lite.exe --demo` opens a clearly labeled offline preview with fictional data. Add `--compact` to start with a 900 × 640 client area. The preview never reads saved credentials, logs in, or writes theme preferences. The ignored `live_account_read_only` integration test requires an existing remembered account in Windows Credential Manager and reads only lists, without printing personal content. It is not run during normal tests.

## Authentication

Starts at `/loguj/portalRodzina`, requests the state-bearing authorization URL without following its next redirect, posts the credentials as a form, then follows the JSON `goTo` response one redirect at a time. Requires a Synergia `oauth_token` cookie applicable to the API path before verifying `/Auth/TokenInfo/`. No credentials are sent to a third-party backend. An active session retains credentials in memory for recovery, including when remembering is disabled; its password buffer is zeroed when that session is dropped. HTTP and GUI dependencies can make temporary copies.

Login failures distinguish a missing session cookie from a verification endpoint rejection. Redirect diagnostics include only HTTP statuses and paths, without query strings or cookie values. Regression tests use a local mock server to check the bootstrap stops before form submission and that intermediate redirect cookies reach subsequent requests.

CAPTCHA and additional verification are reported, not automated. HTTP 200 login pages can contain structured `requiredActions`; password-change and leaked-password notices are recognized and direct you to finish the action on the official website. The app does not change passwords or dismiss these notices. If a new data request encounters an expired session, recovery tries `/refreshToken` and verifies it, otherwise reauthenticates, then repeats the original request once. Recovery is serialized across simultaneous callers, recognizes insufficient scopes separately and reinitializes the messages session. There is no keep-alive timer or background refresh. Failed recovery leaves the last successful data visible.

The login flow is based on observed Librus behavior and independent protocol analysis; it is an unofficial integration and can change. Version 0.3.0 was checked against the remembered account for grades, attendance, message lists, notices, timetable, session renewal and a request after renewal. Private content is not printed by the integration test. Do not commit credentials or student data.

## Compact views

Attendance is grouped by day with O (present), N (absent), NU (excused absence), and Z (released). The checkbox menu accepts multiple types and has select-all/clear actions. A day matches if it contains any selected type; its remaining entries stay visible for context. Unrecognized types and lateness remain available under Other.

Grades use a responsive subject grid with arithmetic/weighted averages. Overflowing subject lists use pages instead of vertical scrolling. Clicking a subject opens every grade and its details; a +N badge indicates grades beyond the compact preview. The 5+ and 6 reflection effect is retained.
