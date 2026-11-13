# ATHANOR — by Veilric

A small native launcher built with Rust and [Anomaly Rune](https://github.com/anomalyco/rune). Tide is its narrow, translucent interface: 65% opacity, an anchored input, and just the result you need. Inspired by Raycast and [dinogomez/Runecast](https://github.com/dinogomez/runecast), with an independent design and implementation.

**Private preview, version 0.1.0.** Linux X11 desktop checks have passed on the development machine. The memory budget is still unmet; [review notes](docs/verification/review.md) list remaining issues. Native Windows CI builds and packages the unsigned x64 preview. Windows GUI acceptance and macOS native acceptance remain pending. See the [Windows portable guide](docs/windows-portable.md). Native Wayland is currently refused rather than presenting a launcher that cannot receive input. See [platform support](docs/platform-support.md) and [remaining release gates](docs/verification/native-checklist.md).

## Run

Build with Rust **1.99.1**, then launch:

```sh
cargo build --locked --release --jobs 2 -p athanor --bin athanor
./target/release/athanor
```

On Linux install `build-essential pkg-config libxkbcommon-dev libwayland-dev`; Vulkan drivers and an X11 compositor are runtime requirements. No WebView, Node, or browser runtime is used. This checkout also has `python3 scripts/cargo-local.py ...` for its staged, workspace-local SDK/tooling. That helper isolates development data under `.local-run/data`; a normally launched binary uses the user's application configuration directory.

Press **Ctrl+Alt+Space** (Windows/Linux) or **Option+Space** (macOS). Launch with `--show` to open immediately. Starting a second instance invokes the existing one. The tray/menu has Show, Settings, and Quit. `athanor --quit` also quits the resident process. Start at login is off by default.

Pin the panel to keep it visible while working elsewhere. After three idle seconds with an empty or whitespace-only query, only the input and unpin control remain. Type to expand immediately. Nonempty results stay visible. Escape clears and hides, including while pinned. Drag the small leading mark to move Tide. Settings include opacity, shortcut, folders, angle mode, display preference, placement reset, and start at login.

On X11, compositor opacity is used by default so 65% translucency also works with NVIDIA surfaces that lack native alpha. This uniformly dims text too. Settings can select native alpha after a restart on compatible drivers. Accessibility names and keyboard navigation are provided; native assistive-technology and IME acceptance still need the checks below.

## Features

| Feature | Examples | Scope |
|---|---|---|
| Apps/files/folders | `firefox`, `proposal` | Name search; default Documents, Desktop, Downloads; configurable roots |
| Calculator | `sqrt(144)+2^5`, `sin(30)`, `log(100)` | Scientific real-number arithmetic; radians/degrees; f64 precision, 12 significant display digits |
| Measurements | `12 km in mi`, `32 F to C`, `1 MiB to KiB` | Length, mass, temperature, area, volume, speed, duration, storage |
| Calendar | `today + 45 days`, `2026-12-25 - 14 days`, `days between 2026-11-12 and 2026-12-25` | Local today, ISO dates, calendar days; no time zones/business calendars |
| Currency | `100 usd to php` | Fiat reference rates from Frankfurter; source date, cache/offline/stale labels |

Use decimal points; comma-sensitive input is explained instead of guessed. Unspecified gallons offer US/imperial choices. Calculator arithmetic does not include symbolic calculus, plotting, or complex numbers. A plain number or filename stays searchable. File search skips hidden, build and cache directories, follows no directory symlinks, caps discovery at 500,000 entries and notifications at 8,000 folders, and shows truncation/watch warnings in settings. Refresh search is available when notifications cannot cover a folder.

Copy, Open, Reveal and Copy path are explicit result actions. Opener helpers run on one bounded worker; a failed or unconfirmed opener preserves the query and shows an error. Queries do not launch anything. No persistent query history, accounts, AI service, telemetry, remote file search, or automatic updater. Currency requests send only currency codes, never the amount, query, or filenames. Rates are reference data, with a daily cache and five-second network timeout; an offline first run does not invent a rate.

The same feature modules are available without a window:

```sh
./target/release/athanor --query 'sqrt(144)+2^5' --json
./target/release/athanor --query '100 usd to php'
ATHANOR_DATA_DIR=/tmp/athanor-dev ./target/release/athanor --show
```

## Architecture and extension

`crates/core` has no Rune dependency. `Feature` objects return typed result data and `Arc<dyn FeatureAction>` objects; action execution uses an `ActionHost` port. Add a module and register its object in `crates/app/src/bootstrap.rs`. The engine, UI and action dispatch do not need another feature-name branch. Rust traits and owned structs provide the requested object-oriented boundaries without inheritance.

`QueryEngine` bounds/cancels work and tags generations. Two workers keep only current pending/completed queries. Search snapshots are immutable, cached to disk and refreshed by filesystem events. A separate bounded worker manages rates. The pure `PanelState` reducer owns idle/focus/dismissal rules; native adapters execute effects. The idle timer is a one-shot wake; hidden mode does not have a periodic app polling loop. See the [approved design](docs/superpowers/specs/2026-11-11-athanor-design.md) and [execution record](docs/verification/execution.md).

## Verify and package

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p athanor-core --locked
sh scripts/check-native.sh
sh scripts/package.sh
```

Native UI tests use Rune's actual interaction runtime. `scripts/check-native-x11.py` adds controlled X11/XTest checks; it creates its own typing sentinel and sends text only to Athanor or that sentinel. It records failures rather than declaring unobserved behavior passed. For Windows/macOS, use the [native checklist](docs/verification/native-checklist.md). Resource measurements and their limits are in [resources.md](docs/verification/resources.md).

Build on each target OS. On macOS, `CARGO_BUILD_TARGET=aarch64-apple-darwin sh scripts/package.sh` or `x86_64-apple-darwin` produces an unsigned `.app` archive. On Windows, `powershell -File scripts/package.ps1` produces an unsigned x86_64 ZIP. Linux packaging produces an x86_64 tarball on the available host. Install target toolchains before cross-architecture macOS builds. The Windows workflow checks formatting, linting, and tests, then compiles with a static C runtime, packages `ATHANOR.exe`, validates the AMD64 PE and distribution files, runs isolated non-GUI queries, and publishes a checksummed workflow artifact. Native CI also checks Linux and both macOS architectures. Signing, notarization, and Windows GUI acceptance remain pending.

Rune is pinned to `bb310fc9e2c4a1a1937814c71efc08924a5c2885`; `Cargo.lock` is preserved. See [dependency notes](docs/dependencies.md). Local packages include dependency license texts and metadata. The official repository is private at [Veilric/ATHANOR](https://github.com/Veilric/ATHANOR). This source has not been assigned a public distribution license.
