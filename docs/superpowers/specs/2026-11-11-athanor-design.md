# Athanor desktop launcher — design

Status: design and implementation plan approved for native execution. Product implementation is in progress. The browser mockups remain visual references, not the native application.

## Intent

Athanor is a small, local-first desktop utility written in Rust with Anomaly's Rune native UI framework. It combines application/file discovery, scientific calculation, measurement conversion, calendar arithmetic, and reference currency conversion in one conversational input. It should make a short task easy while leaving the current workspace visible.

Windows, macOS, and Linux are version-one operating systems. The app has its own identity: the user selected the Tide visual direction, rather than reproducing Raycast's or Runecast's interface.

The confirmed reference is [dinogomez/runecast](https://github.com/dinogomez/runecast), a Rust/Rune launcher. Its documented macOS/Linux support is reference material, not evidence of Athanor's Windows compatibility. No changes to that repository are authorized or necessary.

## Confirmed decisions

- Working product name: Athanor.
- Native Rust plus Anomaly Rune; one resident app process with background workers.
- Independent feature modules with small shared interfaces, using Rust structs, traits, and composition.
- Windows, macOS, and Linux in the initial release. Linux coverage includes X11 and verified layer-shell Wayland desktops; GNOME Wayland is included only after its compatibility path passes typing, topmost, focus, and global-shortcut checks. If it cannot pass, that desktop configuration is explicitly deferred.
- A deterministic conversational input with compact result cards; no AI dependency or account.
- Tide: a slim, right-side, translucent panel at 65% panel opacity by default.
- Global invocation shortcut; Escape and outside clicks dismiss an unpinned panel.
- Pinning keeps the panel visible when the other app regains focus.
- Pinned plus empty query collapses to input only after three idle seconds. Typing immediately expands it. Nonempty queries and results remain expanded.
- An accessible unpin control remains in the compact input. The input stays anchored during expansion/collapse.
- Dismissal clears the query. No persistent query history in version one.
- The shortcut focuses the input. Outside clicks release keyboard focus without hiding a pinned panel. Other apps' typing is never captured by the pinned bar.
- Initial placement near the active monitor's right edge; dragging and placement persistence per monitor. A pinned panel stays where it was placed.
- Local operation except fetching reference currency rates. No automatic updater, account, telemetry, or remote search.

## Approaches considered

### Selected: one resident native app

The Rune UI, query engine, and feature modules share one process. Bounded workers handle file indexing, filesystem queries, network requests, and any calculation that can exceed the interaction budget. The UI thread owns panel state and renders returned results. Workers return data rather than mutate UI state.

This minimizes process overhead and deployment complexity. Adding a feature means implementing a module and registering it in the composition root. A process crash restarts the app as a whole; external third-party code is not loaded in version one.

### Alternative: background core and separate UI process

A long-lived core owns indexing and features; a separate UI communicates with it. Indexing can survive UI restarts and the UI can be released independently. This introduces another process, protocol, lifecycle, and failure path. The user selected the simpler single-process approach for this version.

## Product scope

### App and file search

Discover installed launchable applications through platform integrations. Search file and folder names, not document contents. Start with the platform's Documents, Desktop, and Downloads locations; allow users to add and remove search roots. Honor redirected/localized folders and missing paths. Skip hidden folders and common dependency, build, and cache folders by default.

Results show a recognizable title, type, and concise location, with the full path available when needed to distinguish duplicates. Support keyboard selection, opening the selected result, copying its path, and revealing its location. Opening is an explicit action. No actions execute while typing. Use native icons where economical, with simple local fallback icons.

Matching and ordering must be deterministic. Exact and prefix matches outrank fuzzy matches; stable tie-breaking prevents unnecessary movement. Return a bounded number of results. Do not introduce persistent usage learning or query history in version one. Indexing and stale index recovery must never block typing. Keep a reusable metadata cache and update it incrementally where the platform permits.

### Scientific calculator

Support arithmetic, grouping, unary signs, powers, roots, constants, logarithms, and trigonometry. Examples include (12 + 8) * 3 and sqrt(144) + 2^5. Symbolic algebra, calculus, graphing, and complex-number arithmetic are outside this version.

Expressions use a numerical parser and documented grammar, never a shell or general-purpose code evaluator. Incomplete input should remain editable without alarming error messages. A complete expression with division by zero, a domain error, or overflow gets a concise explanation and no copyable fabricated answer.

Builder defaults: exponentiation is right-associative; -2^2 means -(2^2). Start in radians, provide a degrees/radians setting, and show the angle mode with trigonometric results. Choose and document numeric precision during dependency selection. Formatting must avoid floating-point display artifacts and must not silently turn very small nonzero results into zero; use scientific notation when appropriate. Exact financial/engineering precision is not claimed by the scientific evaluator.

### Measurement conversion

Initial families: length, mass, temperature, area, volume, speed, duration, and digital storage. Support metric and common imperial aliases. Distinguish decimal storage from binary storage; handle temperature offsets correctly. Examples: 12 km in mi and 72 F to C.

Incompatible dimensions produce an explanation. Ambiguous aliases, such as an unspecified gallon, require a choice between supported definitions rather than silently guessing. Duration unit conversion is separate from calendar arithmetic. A unit registry belongs to the conversion module; the UI does not own conversion constants.

### Calendar arithmetic

Support explicit dates, today, addition/subtraction of calendar days, and signed differences between two dates. Examples: today + 45 days; 2026-11-11 + 45 days; days between 2026-11-11 and 2026-12-25. Date differences count elapsed calendar boundaries; an identical date has difference zero. The user's local date determines today; calculations operate on calendar dates rather than adding 24-hour timestamps across daylight-saving changes.

Builder default: ISO dates are the unambiguous explicit input format. Render a readable date and weekday and offer ISO text when copied. Reject impossible dates. Date parsing and the current date/clock must be injectable for deterministic tests. Time-zone conversion, holiday calendars, business-day arithmetic, and calendar integrations are deferred.

### Reference currency conversion

Use daily reference exchange rates from a documented provider, for the fiat currencies that provider supports. Provider selection remains a builder task: verify PHP support, daily publication semantics, usage/attribution terms, and access without a user account before selecting and pinning an integration. Do not imply all currencies are covered. No trading, cryptocurrency, brokerage, or historical rate interface is included.

Display the rate source and provider timestamp, not just the last download time. Cache successful responses locally. Refresh stale data in the background when currency conversion is used, with bounded retries, timeout, and rate limiting. A provider may not publish a new rate every calendar day; preserve its actual timestamp and label stale/offline data honestly. Never block query entry on network access. With no usable cache and no connection, explain that a rate is unavailable. Never ship mock rates as a production fallback.

## UI and interaction

### Visual identity

The approved visual reference is the [standalone Tide preview](../../design/athanor-tide-preview.html), preserved with the design document. Its earlier Ember/Tide comparison remains in the local visual-companion session for reference. Browser HTML is not the production UI or a performance benchmark.

Use smoky blue glass, cool restrained focus accents, system UI typography, tabular numerals for answers, and quiet separators. The default panel alpha is 65%, not 65% transparency. Opacity remains user-adjustable. Keep readable primary/secondary text against varied backgrounds at the selected alpha. Respect reduced motion; there are no decorative idle animations, avatars, greetings, feature tile grids, or a scrim over the desktop.

Input remains at the top of the right-side panel. Results extend downward without shifting the input's anchor. The compact state contains the prompt/input and a small unpin button only; result area, title/header, and footer are hidden. Limit and scroll long results instead of allowing the panel to cover the whole work area.

### Panel state rules

| Situation or event | Required behavior |
| --- | --- |
| Start or restart | Start hidden; restore configuration and placement without stealing focus. Builder default: pin state does not auto-show a panel at startup. |
| Invocation shortcut | Reveal/refocus the existing panel and focus its input; never create a duplicate panel. |
| Unpinned, outside click | Dismiss, clear the query and results, return to the previous app. |
| Pinned, outside click | Remain visible, release focus, preserve a nonempty query and results. |
| Pinned, empty or whitespace query, three idle seconds | Show only the input bar and unpin control. Stop the one-shot idle timer after the transition. |
| Typing a nonempty query into compact input | Expand immediately and request fresh results. |
| Nonempty query becomes idle | Keep the query and results visible; never discard them on a timer. |
| Input cleared while pinned | Restart the three-second idle delay. |
| Pointer/key interaction before empty-panel delay expires | Restart that delay; do not accumulate timers. Focusing an already compact input alone does not expand it. |
| Unpin | Leave compact mode; outside-click dismissal applies again. |
| Escape | Dismiss even when pinned, clear the query/results, and cancel pending query/idle work. |
| Successful copy/open action, unpinned | Perform the explicit action and dismiss. |
| Successful copy/open action, pinned | Retain panel and nonempty query/results; opening an app/file yields focus to its destination. |
| Failed action | Keep input/results and show a recoverable explanation. |

The browser's copy/open simulations do not define every production action transition; use the rules above. Typing in another app never affects the pinned input. UI expansion must not implicitly request focus.

### Placement, focus, and settings

Place near the active monitor's right edge on first reveal. Allow dragging using an affordance that does not conflict with input editing. Persist and clamp geometry per monitor/work area, honoring DPI, taskbars, docks, and display removal. A user-pinned panel remains on its chosen monitor; invoking the shortcut focuses it there. If a platform cannot report the active display, use an explicit preferred-display setting and disclose that limitation rather than silently following an unrelated pointer.

Builder shortcut defaults: Option+Space on macOS; Ctrl+Alt+Space on Windows/Linux. Make shortcuts rebindable, report conflicts, and preserve an existing valid binding if a new registration fails. Provide a tray/menu action to reveal the app and reach settings without the shortcut. Escape must remain available. Do not globally intercept unmodified keys.

Keep settings small: invocation shortcut, opacity, preferred monitor/reset placement, indexed folders, angle mode, and an optional start-at-login toggle (off initially). Provide quit through the tray/menu. No persistent transcript or query-history screen.

## Modules and interfaces

The feature interface is the seam between the query engine and feature implementations. Its small surface covers a stable feature identity, query submission/cancellation, typed result data, and an explicit action request. Queries carry a generation and local context; results carry opaque feature/action identities so a new feature does not require another branch in a central action enum or UI switch.

Five built-in modules implement this interface: search, calculator, measurements, calendar, and currency. The search module may compose separate app-discovery and file-index implementations internally. Register built-ins in one composition root. Future features should usually require their module, its tests, and one registration change. No runtime plugin marketplace, scripting engine, or third-party plugin loader is included.

The query engine classifies explicit syntax deterministically, merges appropriate providers, and produces a bounded result list. Recognized mathematical/conversion queries show their direct answer before incidental filename matches. An ambiguous query must not execute an action or invent a result. Features return display data and explicit actions rather than Rune elements. The UI owns presentation, selection, pinning, timers, and focus rules.

Platform adapters hide native shortcut registration, capability detection, panel hosting/focus, app discovery, clipboard copy, opening/revealing files, and display/work-area discovery. Windows/macOS/Linux vary at this interface; these are real adapters, not speculative abstractions. Feature logic must run in tests without starting a Rune window or requiring a desktop session.

Clock, exchange transport/cache, and filesystem discovery are dependencies supplied to their respective modules. Avoid process-wide mutable feature state. UI-thread ownership plus message passing is preferred over sharing arbitrary locks through the feature interface. Exact crate layout and function signatures are builder decisions provided the interfaces and responsibilities remain small and clear.

## Resource behavior

User-approved acceptance targets: warm resident reveal under 100 ms; near-zero hidden idle CPU; resident app memory budget of 150 MB with 100,000 indexed paths on an ordinary laptop with 8 GB RAM and integrated graphics. These are targets to measure, not established guarantees from the choice of Rust or Rune.

Builder measurement defaults: report p95 warm reveal latency across at least 30 invocations, at rest after indexing; separately report time to first bounded local result, targeting under 100 ms p95 on the same dataset. Report combined app RSS at steady state with the panel hidden, input-only pinned, and expanded; note GPU allocation separately. Measure idle CPU over a quiet minute with no query or indexing task. Track initial indexing time, disk activity, and peak memory separately so they are not hidden by steady-state figures.

Use cancellable/generation-tagged query jobs; old results cannot overwrite a newer query or resurrect dismissed results. Bound worker concurrency, parser work, visible results, icon caches, and file queues. Use incremental indexing and native discovery facilities where useful; avoid rescanning the whole home directory on every keypress. Currency fetching never occurs on the UI thread.

Request repaint when state changes. Hidden and pinned-idle states must not drive continuous frame loops. The three-second collapse uses one cancellable timer, not polling. Disable decorative motion/expensive optional effects before sacrificing correctness or typed-input responsiveness if a target is missed; present measurements and a concrete adjustment rather than claiming a pass.

## OS capability findings and accepted coverage

Rune is pre-release. Pin an exact upstream revision and verify capabilities at that revision before building the full UI. Do not rely solely on stale README descriptions.

- Windows and Linux X11 can use an ordinary transparent, borderless topmost window that accepts typing. macOS can use a suitable native panel. Native desktop restrictions still govern focus and fullscreen presentation.
- Supported Wayland compositors can host keyboard-enabled layer-shell panels when the required protocol/version is present.
- Native GNOME Wayland does not provide that layer-shell path. Rune's XWayland overlay fallback explicitly never takes keyboard focus, and its keyboard-on-demand overlay excludes that fallback. An ordinary Rune Wayland window accepts typing, but its winit backend ignores the always-on-top request. A normal launcher window therefore does not meet the persistent topmost requirement there.
- A separate compatibility option is an ordinary X11 window hosted by XWayland, rather than Rune's special overlay fallback. Winit sends topmost requests and Mutter handles them. Source inspection suggests selecting the X11 event-loop backend at launch is possible, but Rune would then also choose X11 shortcut grabs. Whether invocation works while a native Wayland app has focus remains unverified. Native testing and potentially a portal/desktop shortcut bridge are required before claiming this path works; do not confuse a plausible source-level route with verified support.
- Wayland global shortcuts use the GlobalShortcuts portal; availability and user permission must be handled. A missing portal is not the same as a shortcut conflict. Activation tokens must be forwarded for focus.
- Universal display above exclusive fullscreen apps, secure desktops, OS permission prompts, and lock screens is not technically promised. The product target is ordinary active application windows; OS-controlled surfaces remain subject to platform policy.

Accepted user decision: the initial release targets Windows, macOS, Linux X11, and verified layer-shell Wayland desktops. GNOME Wayland is included only after an XWayland compatibility path passes typing, topmost, focus, and global-shortcut checks. If it cannot pass, explicitly defer that desktop configuration without blocking the other supported platforms' release. Do not quietly describe a non-topmost fallback as equivalent support. Native GNOME Wayland support can remain deferred if the compatibility path meets the requirements.

Primary sources checked:

- [Rune overlay options and GNOME fallback](https://github.com/anomalyco/rune/blob/dev/crates/rune/src/overlay/options.rs#L128)
- [Rune layer-shell keyboard support](https://github.com/anomalyco/rune/blob/dev/crates/rune/src/platform/layer_shell.rs#L502)
- [winit Wayland ignores window level](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/linux/wayland/window/mod.rs#L430)
- [Rune global shortcut behavior](https://github.com/anomalyco/rune/blob/dev/crates/rune/src/shortcuts/README.md)
- [Rune pre-release revision policy](https://github.com/anomalyco/rune#add-it)
- [winit backend selection](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/linux/mod.rs#L734)
- [winit X11 topmost requests](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/linux/x11/window.rs#L1073)
- [Mutter X11 topmost handling](https://github.com/GNOME/mutter/blob/main/src/x11/window-x11.c#L3390)

The inspected local environment is Ubuntu GNOME X11 on x86_64 with Rust/Cargo 1.99.1. Rune's required xkbcommon and Wayland development packages were not found by pkg-config. Installing system dependencies is an implementation setup task and may require system authorization. Only a Linux build target is presently installed. Native Windows/macOS testing requires appropriate machines or runners; Linux headless screenshots cannot establish those platforms' behavior.

## Verification and delivery

Build tests around public feature interfaces and real user behavior: numerical precedence/domain cases, tiny/large number formatting, dimensional compatibility, affine temperatures, storage units, leap dates and DST-independent date arithmetic, provider timestamp/cache/offline behavior, filename ranking and exclusions, cancellation races, and action dispatch to the correct feature. Use representative fixtures and temporary directories; do not create tests that merely repeat conversion tables or trivial getters.

Exercise panel state transitions including Escape, focus loss, idle timer cancellation, unpinning from compact mode, query preservation, monitor removal, and late query completion. Validate accessibility roles, keyboard navigation, input-method composition, contrast at 65%, DPI, and reduced motion. Use Rune's headless rendering for repeatable UI checks, followed by native desktop smoke tests for focus, transparency, topmost behavior, shortcut registration, drag placement, and resume from sleep.

Every claimed supported OS/desktop must build and pass its native smoke checks. Publish a capability/verification matrix rather than implying coverage from a successful Linux build. Test the measured resource targets on representative hardware. Functional correctness and accessible keyboard operation are release requirements.

Implementation sequence to plan after design approval: first validate native panel/shortcut/alpha/focus capabilities on the chosen platform matrix; then build the feature/query interfaces and modules; integrate Tide and its state machine; measure resources and package native artifacts. The initial platform check is a small production feasibility milestone, not permission to replace Rune silently.

Deliver local source, documentation, relevant tests, and native package/build instructions. Builder default package targets: Windows x86_64, Linux x86_64, and macOS arm64/x86_64, subject to the verified Rune revision's supported minimum OS versions. Extra CPU architectures are deferred. Confirm minimum OS versions against the pinned dependencies before claiming release support. Native signing/notarization and external distribution depend on available user credentials and are not silently invented.

No repository creation, push, PR, publication, or external write is authorized by this design. Any later GitHub writes require verified Veilric ownership and authentication under the user's account policy. This workspace currently has a placeholder .git directory rather than a usable repository, so the design is saved locally without a Git commit.

## Out of scope

AI chat, accounts, sync, telemetry, web search fallback, clipboard history, snippets, window management, workflow automation, runtime plugins, symbolic mathematics, graphing, crypto/trading, timezone/world-clock interfaces, file-content indexing, and automatic updating are outside version one. Runecast's extra features do not expand Athanor's approved scope.

## Builder discretion and remaining approval

Builder decisions include exact Rust signatures/crate organization, numerical and units libraries after validating behavior and licensing, index/cache storage format, bounded worker implementation, rate-provider selection meeting the constraints above, native packaging details, and small accessibility/spacing adjustments that preserve Tide and 65% alpha. Defaults explicitly identified above are proposed engineering choices, not earlier user quotations.

The Linux compatibility choice is settled. Final approval of this design is required before scaffolding, dependency installation, product code, or the implementation plan. All consequential user decisions are now recorded; remaining implementation choices are assigned to the builder above.

Builder review completed: no other unanswered questions materially block implementation planning. Its sole remaining product question was the Linux capability choice, now answered and recorded above.
