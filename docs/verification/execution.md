# Implementation and verification record

Local source preview created 2026-11-12 following the approved design and implementation plan. No Git repository was initialized, remote changed, commit/push made, repository created, or workflow remotely executed. The existing `.git` is a read-only placeholder. The ignored durable ledger is `.superpowers/sdd/2026-11-12-athanor-implementation/progress.md`.

The native executable is `target/release/athanor`. Local package scripts include source documentation and locked dependency license metadata/texts. Windows/macOS package scripts and CI definitions are prepared, without claiming a successful native build on those systems.

## Verified

- Scientific arithmetic, eight unit families, calendar-day arithmetic, fiat-rate cache and error behavior, installed-app/file search, cancellation and custom feature/action extensibility.
- Full workspace tests: 65 passed, three intentionally ignored external/native tests. The local IPC integration test, native X11 notification test and live Frankfurter PHP contract were also run explicitly and passed.
- Rustfmt check, Clippy across the workspace/all targets with warnings denied, portable core tests, locked release build.
- Real Rune UI interaction tests and sixteen scene renders (eight states at 1× and 2×). Calculator/search/settings/compact renders inspected visually.
- Controlled real X11 topmost, typing, native calculator clipboard, input anchor, pinned 3-second idle, outside typing, query retention, Escape and 30 shortcut checks.
- Index access-event regression, corrupt-settings recovery on later saves, allocation sharing and compact/lossless path-cache regressions were each reproduced failing before their fix.

Native acceptance gaps and resource limits are explicit in [native-checklist.md](native-checklist.md), [resources.md](resources.md) and [platform-support.md](../platform-support.md). This is a functional local preview, not a declaration that every platform/performance release gate has passed.

## Execution decisions

1. Preserve the placeholder `.git` and use the current workspace; no Git rollback or commit-range evidence exists.
2. Retain the ignored ledger/artifacts because no Git history can preserve this execution record; local disk usage remains.
3. Use the exact installed stable compiler alias plus local caches/SDK/tools, keeping the project's named 1.99.1 pin; this sandbox needs the helper environment.
4. Extend the rates port with a default error accessor so provider/offline errors can be shown; one extra defaulted method.
5. Add per-data-directory OS locking and token-gated loopback Show/Settings/Quit forwarding; one blocking listener thread, no query/file IPC.
6. Suspend idle collapse in settings; one extra reducer event/flag prevents settings disappearing during editing.
7. Use compositor opacity on X11 by default for NVIDIA compatibility, with native alpha selectable; text also dims and a compositor is required.
8. Keep native Wayland disabled and Windows/macOS explicitly unverified; this preview does not yet constitute the full multi-platform release. Layer-shell hosting and macOS live-display refresh remain work, with actual platform acceptance required.

The [independent review and corrections](review.md) records six Important fixes and three deferred minor findings. All execution rulings and native/resource limits remain explicit.

9. Add a background-action policy and bounded action worker to confirm opener failures asynchronously; one default trait method, a thread-safe host bound and one worker thread.
10. Add Linux XRandR/work-area and login1 resume notifications; two native-service threads, with direct optional use of Rune's existing DBus/runtime dependencies.
11. Keep Windows/macOS automatic display/resume observers as implementation/release gates; invocation recovery is not equivalent to automatic recovery.
12. Retain unverified Windows/macOS native execution/package status; native failures on those targets remain possible.
13. Retain Wayland startup refusal until hosting/compatibility is implemented and tested; those sessions cannot use this preview natively.
14. Retain physical IME/accessibility/mixed-DPI/sleep gates; synthetic/pure tests cannot certify hardware integration.
15. Finish local resource/package checks but retain the failed memory budget; this desktop cannot certify the 8GB integrated-GPU target.
16. Leave signing, publication and Git integration untouched; delivery has no signed installer, remote history or public download.
