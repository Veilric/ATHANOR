# Pinned dependency notes

Rust 1.99.1 / edition 2024 is pinned in rust-toolchain.toml. Cargo.lock is the dependency resolution record. Local development staged exact-version rustfmt/Clippy and Linux SDK files under `.local-run`, with official archive SHA256 verification; no host packages/toolchains were replaced.

| Dependency | Purpose | Published license |
|---|---|---|
| Anomaly Rune, revision bb310fc9 | Native UI/window/keyboard/rendering | MIT |
| meval 0.2.0 | Scientific expression evaluation | Unlicense OR MIT (MIT selected) |
| chrono 0.4 | Calendar arithmetic/local date | MIT OR Apache-2.0 |
| rust_decimal 1 | Decimal fiat rates/amounts | MIT |
| serde / serde_json 1 | Bounded configuration/cache serialization | MIT OR Apache-2.0 |
| ureq 3.5.1 / rustls | Bounded blocking HTTPS on the rate worker | MIT OR Apache-2.0 / Apache-2.0 OR ISC OR MIT |
| notify 8 | Event-driven filesystem updates | CC0-1.0 |
| directories 6 | Platform configuration/user folder paths | MIT OR Apache-2.0 |
| arboard 3.7 | Explicit clipboard writes | MIT OR Apache-2.0 |
| getrandom 0.4 | Local IPC token | MIT OR Apache-2.0 |
| tempfile 3 | Atomic file replacement | MIT OR Apache-2.0 |
| x11rb 0.13 | Linux monitor/work-area/compositor adapter | MIT OR Apache-2.0 |
| zbus 6 / tokio 1 / futures-util 0.3 | Linux login1 resume signals, reusing Rune's runtime dependencies | MIT / MIT / MIT OR Apache-2.0 |
| tray-icon 0.26.1 | macOS status menu (Rune supplies Linux/Windows tray) | MIT OR Apache-2.0 |

Package generation exports the locked **full** dependency metadata and available license files, including platform/optional packages. It does not assign a distribution license to Athanor itself. meval's transitive nom 1.2.4 emits a future Rust incompatibility notice on the pinned compiler; current builds/tests/Clippy pass. Replacing the parser is a future maintenance consideration, requiring the existing precedence/number tests to keep passing.

Rates use [Frankfurter v2](https://frankfurter.dev/) blended pair data and provider attribution. There is no API key. The live PHP contract test is opt-in and separate from deterministic tests. Fixtures are for tests only. No exchange rate is fabricated for an offline application.
