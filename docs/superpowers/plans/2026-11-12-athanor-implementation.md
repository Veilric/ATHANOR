# Athanor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Build the native Rust/Rune launcher with the five settled features and Tide's 65% opacity, including its input-only pinned idle state.

**Architecture:** One resident process, a Rune-independent core, and native platform adapters. Feature objects return result data and executable action objects; the UI does not switch on feature names. Bounded workers perform indexing, queries, and exchange-rate I/O.

**Tech Stack:** Rust 1.99.1, Anomaly Rune pinned to bb310fc9e2c4a1a1937814c71efc08924a5c2885, serde/serde_json 1, chrono 0.4, meval 0.2.0, rust_decimal 1, ureq 3.5.1, directories 6, notify 8. Resolve and preserve Cargo.lock; verify these dependencies' published licensing and APIs during setup. Do not use the unrelated rune 0.14 scripting crate already present in the local cache.

**Spec:** [Athanor design](../specs/2026-11-11-athanor-design.md). [Approved standalone interface](../../design/athanor-tide-preview.html).

**Execution checkpoint (2026-11-12):** The local preview, native X11 checks, release build and feature modules are implemented. Windows/macOS native verification, layer-shell hosting, macOS live-display refresh, physical monitor/IME/sleep checks and the 150 MB acceptance target remain open release gates. See docs/verification/execution.md and the durable ledger; unchecked items below retain their unmet native/release requirements.

## Global Constraints

- Working product name: Athanor.
- Native Rust plus Anomaly Rune; one resident app process with background workers.
- Tide: a slim, right-side, translucent panel at 65% panel opacity by default.
- Pinned plus empty query collapses to input only after three idle seconds. Typing immediately expands it. Nonempty queries and results remain expanded.
- Dismissal clears the query. No persistent query history in version one.
- Local operation except fetching reference currency rates. No automatic updater, account, telemetry, or remote search.
- Initial release: Windows, macOS, Linux X11, and verified layer-shell Wayland desktops. GNOME Wayland is included only after its compatibility path passes typing, topmost, focus, and global-shortcut checks; otherwise explicitly defer that desktop configuration.
- Acceptance targets: warm resident reveal under 100 ms; near-zero hidden idle CPU; resident app memory budget of 150 MB with 100,000 indexed paths on an ordinary laptop with 8 GB RAM and integrated graphics.
- Source and packages remain local. Do not initialize or replace the placeholder .git, change remotes, push, create a repository, or publish. Commits are conditional on a later authorized, usable checkout; this plan does not require them in the current workspace.
- Native GUI tests require the missing Linux development packages and suitable desktop access. Windows/macOS support is not verified by a successful Linux build or headless rendering.

## Review Focus

1. Unicode/non-UTF-8 filenames, disappearing files, unreadable directories, and symlink cycles must not crash search or invoke a shell. Task 6 owns these tests.
2. Slow old queries or exchange replies arriving after Escape/new input must never restore dismissed content or replace newer results. Tasks 2, 7, and 9 own these tests.
3. Input-method composition and a pinned bar without focus must leave typing in the active app untouched. Tasks 8 and 10 own these tests.
4. Sleep/resume, display removal, and mixed DPI must not strand an invisible focused panel offscreen. Tasks 8 and 9 own these tests.
5. Locale, ambiguous units, tiny/huge numbers, and stale/malformed rate data must produce clear results instead of silent rounding to zero or fabricated answers. Tasks 3–5 and 7 own these tests.

## File structure and shared interfaces

Two workspace members: crates/core (package athanor-core, no Rune dependency) and crates/app (package/binary athanor, optional native-ui feature enabled by default). Core tests run without compiling the UI. The native dependency may still need fetching when Cargo resolves the workspace; do not confuse dependency resolution with GUI compilation.

- Root: Cargo.toml, Cargo.lock, rust-toolchain.toml, README.md, .gitignore.
- Core: src/lib.rs, feature.rs, query.rs, cancel.rs, number.rs; features/calculator.rs, measurements.rs, calendar.rs, currency.rs, search/mod.rs, search/rank.rs, search/index.rs; panel.rs and geometry.rs.
- App: src/main.rs, bootstrap.rs, config.rs; services/query_worker.rs, file_index.rs, rate_store.rs, rate_http.rs; platform/mod.rs, linux.rs, macos.rs, windows.rs, capabilities.rs; ui/mod.rs, tide.rs, settings.rs, tokens.rs, scenes.rs.
- Probe: crates/app/src/bin/athanor-probe.rs, requiring native-ui.
- Tests: crates/core/tests/{feature_contract,calculator,measurements,calendar,search,currency,panel,geometry}.rs and crates/app/tests/{platform_policy,config,rate_store,query_worker,ui}.rs.
- Delivery: assets/icon.svg, scripts/check-native.sh, scripts/check-native.ps1, scripts/measure-resources.py, scripts/package.sh, scripts/package.ps1, .github/workflows/check.yml, docs/platform-support.md, docs/verification/native-checklist.md, docs/verification/resources.md.

Lock these core names in Task 2; use them unchanged in later tasks:

    pub struct QueryRequest { pub generation: u64, pub text: Arc<str> }
    pub struct QueryContext { pub today: NaiveDate, pub angle_mode: AngleMode }
    pub trait Feature: Send + Sync {
        fn id(&self) -> &'static str;
        fn query(&self, request: &QueryRequest, context: &QueryContext,
                 cancel: &Cancellation) -> FeatureReply;
    }
    pub enum FeatureReply {
        NotApplicable, Incomplete, Results(Vec<ResultItem>),
        Invalid { message: String }
    }
    pub struct ResultItem {
        pub key: ResultKey, pub title: String, pub detail: String,
        pub presentation: Presentation, pub actions: Vec<Arc<dyn FeatureAction>>
    }
    pub struct ResultKey { pub feature: &'static str, pub local: String }
    pub enum Presentation { Answer, Application, File, Folder, Explanation }
    pub trait FeatureAction: Send + Sync {
        fn label(&self) -> &str;
        fn execute(&self, host: &dyn ActionHost) -> Result<ActionOutcome, ActionError>;
    }
    pub trait ActionHost {
        fn copy_text(&self, text: &str) -> Result<(), ActionError>;
        fn open(&self, target: &OpenTarget) -> Result<(), ActionError>;
        fn reveal(&self, path: &Path) -> Result<(), ActionError>;
    }
    pub enum OpenTarget { Path(PathBuf), Application(String) }
    pub struct ActionOutcome { pub yield_focus: bool }
    pub struct Cancellation(Arc<AtomicBool>);
    pub fn Cancellation::cancel(&self);
    pub fn Cancellation::is_cancelled(&self) -> bool;
    pub struct QueryBatch { pub generation: u64, pub items: Vec<ResultItem> }
    pub fn QueryEngine::new(features: Vec<Arc<dyn Feature>>) -> Result<Self, RegistryError>;
    pub fn QueryEngine::query(&self, request: &QueryRequest,
        context: &QueryContext, cancel: &Cancellation) -> QueryBatch;

AngleMode, ActionError, and RegistryError are small local enums/structs defined alongside these interfaces. Result order uses registration priority and stable result keys. Feature-specific actions implement FeatureAction, so adding a feature does not require a central action dispatch branch.

Limits: 4,096 input bytes, parser nesting 64, at most 50 visible results, two query workers with a latest-only bounded pending queue. Applying a batch requires its generation to equal the active generation and the panel to be visible. Invalidating a generation happens before clearing/dismissing.

## Task 1: Reproducible workspace and native capability probe

**Files:** Root manifests/toolchain; crates/core/src/lib.rs; crates/app/{Cargo.toml,src/main.rs,src/platform/capabilities.rs,src/bin/athanor-probe.rs,tests/platform_policy.rs}; docs/platform-support.md.

**Interfaces:** Produces PanelBackend and CapabilityReport, with fn meets_overlay_requirements(&self) -> bool. A report distinguishes unverified, verified, and unsupported; configuration alone is never a successful observation.

- [ ] Write platform_policy tests: an ordinary native Wayland window without layer-shell is not advertised as a working topmost panel; a missing shortcut/focus observation makes the report incomplete.
- [ ] Scaffold only the manifests and test surface needed to run cargo test -p athanor --no-default-features --test platform_policy. Confirm the assertions fail before implementing the capability policy.
- [ ] Implement the policy and a small typed-input probe using the pinned Rune example. Enable Rune accessibility, dbus, metal-backend, svg, and native-file-dialog, disabling default features to omit optional audio/hot reload. Use a transparent, borderless, initially hidden, topmost host with a real shortcut. Explicitly remove Rune's 420×400 default minimum for the compact bar.
- [ ] Resolve/preserve Cargo.lock. Run the policy tests, cargo test -p athanor-core, and cargo check -p athanor --all-targets. Native builds require libxkbcommon-dev and libwayland-dev locally; install only through an authorized environment action. If denied, preserve the precise failure and continue portable core work.
- [ ] Run the probe on available native desktops; document observation and commands in the support matrix. Prepare Windows/macOS checks without marking them passed. A failed essential local host capability blocks claiming that host supported; pending external runners need not block independent core implementation.

## Task 2: Feature objects, deterministic query ordering, and cancellation

**Files:** Core feature.rs, query.rs, cancel.rs, tests/feature_contract.rs.

**Interfaces:** Produces every shared interface above. Test helpers include FakeHost recording copy/open/reveal calls and fake Feature implementations.

- [x] Write tests registering two arbitrary features, invoking a returned custom action through FakeHost, rejecting duplicate feature IDs, honoring cancellation, and keeping ordering stable across identical queries. Assert an invalidated generation cannot be applied.
- [x] Run cargo test -p athanor-core --test feature_contract; verify the new behavior fails before implementation.
- [x] Implement the interfaces and QueryEngine. Explicit answers rank before incidental filename hits. Ignore empty/incomplete provider output rather than inventing a result. Bound output to 50; do not execute actions during queries.
- [x] Run the feature_contract tests. Add a third test-only feature and prove no engine/UI dispatch edits are needed to expose its result/action.
- [x] Review exported interfaces against the spec; keep Rune, filesystem side effects, and network I/O out of feature dispatch.

## Task 3: Scientific arithmetic and safe number display

**Files:** Core number.rs, features/calculator.rs, tests/calculator.rs.

**Interfaces:** Produces CalculatorFeature::new(), AngleMode::{Radians,Degrees}, fn format_number(value: f64) -> Result<String, NumberError>. Implements Feature.

- [x] Add assertions: sqrt(144)+2^5 → 44; 2^3^2 → 512; -2^2 → -4; sin(30) in degrees → 0.5; log(100) → 2. Test incomplete input, 1/0, sqrt(-1), cancellation, excessive nesting, 0.1+0.2 display, and 1e-12 staying nonzero. Plain filenames and standalone numbers remain searchable.
- [x] Run cargo test -p athanor-core --test calculator and confirm failures.
- [x] Wrap meval 0.2.0, using its Unlicense/MIT licensing option and an explicit function context. Add log as base-10, ln as natural log, and mode-aware trig. Normalize ×/÷ and Unicode minus, not arbitrary x characters inside filenames. Limit parsing before evaluation; reject nonfinite answers. Use documented significant-digit formatting and scientific notation for very small/large values.
- [x] Run calculator tests, including public action copy through FakeHost. If the library's precedence differs, adapt or replace it with a documented equivalent before committing to its grammar.
- [x] Document scientific f64 precision and angle mode in the feature help; do not present raw binary floating artifacts.

## Task 4: Measurements with explicit ambiguity

**Files:** Core features/measurements.rs, tests/measurements.rs.

**Interfaces:** Produces MeasurementFeature::new(), implementing Feature; its internal unit registry owns aliases, dimensions, scale, and affine offset.

- [x] Test 12 km in mi ≈ 7.45645; 32 F to C = 0; 1 MiB to KiB = 1024; decimal MB differs from MiB; US/imperial gallons differ; unspecified gal offers clarification; mass-to-length is rejected. Include area, volume, speed, duration, negative temperatures, and locale-sensitive ambiguity.
- [x] Run cargo test -p athanor-core --test measurements; confirm the new assertions fail.
- [x] Implement the eight agreed unit families, numeric-plus-unit grammar, and independent conversion actions. Accept common singular/plural aliases. Keep ambiguous units noncopyable until resolved. Reuse the number formatter; do not round intermediate calculations.
- [x] Rerun measurements and calculator tests. Exercise actual representative equivalences rather than duplicating the entire constant table in tests.
- [x] Document supported aliases and decimal/binary storage notation in README feature help.

## Task 5: Local calendar arithmetic

**Files:** Core features/calendar.rs, tests/calendar.rs.

**Interfaces:** Produces CalendarFeature::new(), implementing Feature using QueryContext.today; no system clock reads inside the feature.

- [x] With today fixed at 2026-11-11, assert today + 45 days → 2026-12-26; 2024-03-01 - 1 day → 2024-02-29; same-date difference = 0; reversed dates give a negative difference; 2026-02-30 is rejected. Date arithmetic across daylight-saving boundaries still counts calendar days.
- [x] Run cargo test -p athanor-core --test calendar; verify failures first.
- [x] Implement ISO dates, today, ± integer calendar days, and days between ISO dates with chrono NaiveDate checked arithmetic. Display readable date/weekday; the copy action returns ISO text.
- [x] Run calendar tests and verify a date query outranks incidental numeric math without breaking scientific expressions.
- [x] Document accepted date grammar. Do not add time zones/business-day calendars.

## Task 6: Bounded app/file discovery and cached search

**Files:** Core features/search/{mod,rank,index}.rs, tests/search.rs; app services/file_index.rs.

**Interfaces:** Produces SearchEntry { id: String, title: String, location: PathBuf, target: OpenTarget, is_folder: bool }; SearchFeature::new(snapshot: Arc<RwLock<Arc<SearchSnapshot>>>); fn build_file_snapshot(roots: &[PathBuf], cancel: &Cancellation) -> Result<SearchSnapshot, IndexError>. Native adapters supply installed-app entries in Task 9.

- [x] Build temporary-directory tests for exact/prefix/fuzzy order, duplicate titles with different paths, Unicode/non-UTF-8 names, permission failures, symlink cycles, deleted files, excluded caches, cancellation, and repeated searches without filesystem rescans.
- [x] Run cargo test -p athanor-core --test search; confirm failures.
- [x] Implement a compact immutable metadata snapshot and deterministic ranking. Stream a versioned JSON-lines disk cache, swap snapshots atomically, and skip symlink traversal/hidden/build/cache directories. Watch configured roots using notify, coalesce changes, and rebuild affected subtrees on a worker. Watch overflow triggers a bounded background rescan. Return at most 50 rows.
- [x] Run search tests and a generated 100,000-entry query timing check. Verify that open/reveal/copy-path use explicit actions and argument-safe platform calls; no shell interpolation or content indexing.
- [x] Measure snapshot memory and cold indexing separately from warmed search. Supply generic icons until bounded native icon caching is available.

## Task 7: Cached reference currency conversion

**Files:** Core features/currency.rs, tests/currency.rs; app services/{rate_store,rate_http}.rs, tests/rate_store.rs; app tests/fixtures/frankfurter-pair.json.

**Interfaces:** Produces RateSnapshot { base: String, quote: String, rate: Decimal, observed_on: NaiveDate, fetched_at: DateTime<Utc>, source: String }; trait Rates: Send + Sync { fn snapshot(&self, base: &str, quote: &str) -> Option<RateSnapshot>; fn request_refresh(&self, base: &str, quote: &str); }; CurrencyFeature::new(rates: Arc<dyn Rates>). The store accepts injected clock/transport for tests.

- [x] Write tests for cached USD→PHP calculation, no-cache offline explanation, stale label based on source date, malformed/zero/negative/future-dated rate rejection, one refresh per simultaneous pair request, timeout, failed-refresh preservation of good cache, and a late refresh not reviving a dismissed query.
- [x] Run currency and rate_store tests with a fake transport and fixed clock; verify failures.
- [x] Implement Frankfurter v2 blended pair requests with ureq on a worker. The verified response fields are date, base, quote, rate, and optional expanded providers. Request the pair endpoint with expand=providers, preserving its date and source. Parse rate/amount as Decimal. Fetch/cache the current currency catalog only on currency use and admit entries whose type is currency; exclude crypto/metals. Use a five-second global HTTP timeout, 256 KiB response limit, 24-hour refresh TTL, 60-second failure cooldown, and one in-flight refresh per pair. Never send the full query, filenames, or the conversion amount.
- [x] Persist successful cache changes atomically. Run the tests, including weekend-old source dates and corrupted cache recovery without fake replacement rates. Mark rates older than 72 hours stale while always displaying their actual date and cached/offline state.
- [x] Run an opt-in live contract check for PHP and source metadata, clearly separate from deterministic unit tests. Store an attributed fixture for testing only; it is never the app's fallback rate.

## Task 8: Panel state, focus policy, and monitor geometry

**Files:** Core panel.rs, geometry.rs, tests/{panel,geometry}.rs.

**Interfaces:** Produces PanelState and PanelEvent, fn PanelState::reduce(&mut self, event: PanelEvent, now: Duration) -> Vec<PanelEffect>. Effects describe focus/show/hide/resize, cancellation, and one-shot timer scheduling; native execution belongs to Task 9. Geometry uses logical offsets plus monitor identity/work area.

- [x] Test the entire spec transition table with a fake monotonic clock: 2,999 ms remains expanded and 3,000 ms collapses only when pinned/empty; typing expands; nonempty queries survive idle; outside clicks yield focus while pinned; Escape clears/hides/cancels; unpin is reachable from compact mode. Add composing-text events and ignored idle events from other applications.
- [x] Run panel and geometry tests; verify failures.
- [x] Implement the reducer, generation invalidation, and one cancellable idle deadline. Do not use periodic polling. Copy/open success dismisses only when unpinned; failures preserve input. Keep the input anchor fixed while content height changes.
- [x] Add mixed-DPI, negative-origin display, removed-monitor, sleep/resume, and stored-offscreen-position cases; clamp to a surviving work area without forcing focus. Rerun tests.
- [x] Review action/focus effects against the spec, particularly pinned opening of another app and input-method composition.

## Task 9: Native host, configuration, workers, and platform actions

**Files:** App bootstrap.rs, config.rs, services/query_worker.rs, platform/{mod,linux,macos,windows}.rs, tests/{config,query_worker}.rs; support matrix.

**Interfaces:** Produces platform ActionHost, installed-app discovery, shortcut registration, panel effect execution, and QueryWorker::submit(QueryRequest, QueryContext). QueryWorker sends QueryBatch messages back through Rune's waker. Config holds opacity, shortcut, angle mode, search roots, preferred display, positions, and start-at-login.

- [ ] Write tests for bounded latest-query behavior, late/out-of-order replies after Escape, invalid config fallback without deleting user data, persisted 0.65 opacity, shortcut failure preserving the old binding, and unsupported capability reporting. Run them red before implementation.
- [ ] Implement native integrations. macOS: use Rune's launcher-panel options and native app discovery; Windows: enumerate Start-menu links and AppsFolder launchable apps; Linux: parse XDG application entries with proper Exec field-code/argument handling. Opening/revealing uses native APIs or direct process arguments, never a shell.
- [ ] Connect two query workers, one coalesced indexing worker, and bounded rate I/O. Create no continuous idle polling. Load local configuration/cache through directories, supporting ATHANOR_DATA_DIR for isolated dev/test data. Do not read unrelated CLI credentials or authentication files. Start hidden and unpinned; start-at-login is opt-in.
- [ ] Wire shortcuts (Option+Space on macOS; Ctrl+Alt+Space elsewhere), tray/menu show/settings/quit, drag persistence, focus loss, and one-shot timer effects. Layer-shell hosts require actual keyboard/input capability checks. GNOME's ordinary XWayland path is opt-in until it passes the native checklist, including invocation while another native Wayland app is focused.
- [ ] Run app tests, core regression tests, the native probe, and desktop smoke checks. Document unverified platforms. Preserve pending Windows/macOS checks as explicit release gates while independent work continues.

## Task 10: Production Tide view and settings

**Files:** App ui/{mod,tide,settings,tokens,scenes}.rs, tests/ui.rs, assets/icon.svg.

**Interfaces:** Produces fn tide_view(state: &AppState, cx: &mut rune::Cx<AppState>) -> rune::El and settings_view using Task 9 state/effects. AppState owns PanelState, current QueryBatch, Config, and worker handles.

- [ ] Write headless Rune interaction tests: pin/clear/idle produces only input and unpin; typing restores results without moving the input; Escape clears; a nonempty query retains results; selection/copy dispatches the chosen action object. Check visible accessibility names and input composition.
- [ ] Run cargo test -p athanor --test ui with native-ui enabled; confirm behavioral failures before implementing views.
- [ ] Translate the approved Tide mockup into Rune: 0.65 base alpha, cool focus color, readable text, restrained borders, system UI typography, tabular results, no scrim or idle animation. Use approximately 330 logical px width, 53 logical px compact height, and a scrollable expanded result area clamped to the available work area. Keep the unpin affordance accessible. Implement only the agreed small settings surface.
- [ ] Render deterministic expanded, search, currency-offline, error, settings, and pinned-idle scenes at 1×/2×. Compare to the approved preview and inspect contrast over light/dark backgrounds at 65%. Respect reduced motion; cap icon cache at 64 entries.
- [ ] Run headless tests and real desktop checks for typing, IME, focus, shortcut, dragging, topmost behavior, monitor changes, and no continued frame loop while hidden/idle. A headless screenshot alone does not pass native focus tests.

## Task 11: Packages, performance evidence, and handoff

**Files:** README.md; scripts/check-native.{sh,ps1}, measure-resources.py, package.{sh,ps1}; .github/workflows/check.yml; docs/verification/{native-checklist,resources}.md; support matrix.

**Interfaces:** Produces local packages and repeatable verification commands. Native checks record OS/backend/revision, date, and observed pass/fail/unverified status.

- [ ] Add release smoke assertions for fresh startup hidden, five representative feature queries, corrupt-cache recovery, sleep/resume, and every required panel transition. Native scripts must fail if essential typing/focus/topmost/shortcut checks fail, rather than report generic success.
- [ ] Run cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace; cargo build -p athanor --release --locked. Also run core tests without the GUI dependencies. Fix failures before packaging.
- [ ] Measure p95 warm reveal over at least 30 invocations, p95 first local results, steady-state RSS in hidden/compact/expanded states, a quiet minute of idle CPU, and separate 100,000-entry indexing peak/time/disk costs. Report actual numbers, hardware, OS, and GPU allocation separately; do not claim targets passed without evidence.
- [ ] Produce Windows x86_64, Linux x86_64, and macOS arm64/x86_64 build/package instructions and available local artifacts. Prepare local CI definitions; do not push/run remote workflows without later authorization. Verify dependency minimum OS versions before claiming support. Signing/notarization remains conditional on actual credentials.
- [ ] Review the full change against the spec, record remaining native/platform verification gaps, and link the source, executable/package, documentation, tests, and measured evidence. Do not mark the overall app complete while required feature work is missing.

## Planning evidence and self-review

- The Rune revision was resolved from the upstream dev branch, and its toolchain/options/features were inspected directly. [Pinned options](https://github.com/anomalyco/rune/blob/bb310fc9e2c4a1a1937814c71efc08924a5c2885/crates/rune/src/window/spec.rs), [pinned feature flags](https://github.com/anomalyco/rune/blob/bb310fc9e2c4a1a1937814c71efc08924a5c2885/crates/rune/Cargo.toml).
- meval's documented functions/operator support and published package license were checked; the precedence and formatting assertions above remain required. [Library documentation](https://docs.rs/meval/0.2.0/meval/), [package metadata](https://crates.io/api/v1/crates/meval/0.2.0).
- Frankfurter documents its no-key blended feed and caching policy. PHP coverage and the live pair schema were verified through its public endpoints; the fixture remains test-only. [Provider documentation](https://frankfurter.dev/), [PHP metadata](https://api.frankfurter.dev/v2/currency/php).
- ureq's blocking requests run exclusively on workers, with explicit timeouts. [Client documentation](https://docs.rs/ureq/3.5.1/ureq/).
- Coverage check: the spec's features map to Tasks 3–7; module extensibility/cancellation to Task 2; panel/focus/placement to Tasks 8–10; native feasibility to Tasks 1/9; performance/platform/package evidence to Task 11.
- Interface check: all later tasks use the shared Feature/FeatureAction/ActionHost, QueryRequest/QueryContext/QueryBatch, Cancellation, and PanelState names defined above. Feature implementations remain independent of Rune.
- Review Focus check: each of the five listed failure classes has an owning test task. No plan step authorizes a repository write, upstream modification, fake production rates, AI service, or additional product feature.
- Execution preference: native implementation in this session is recommended because these tasks share interfaces and the initial scaffold is small; this avoids a fresh implementation context for each task. A final independent review remains appropriate before delivery.
