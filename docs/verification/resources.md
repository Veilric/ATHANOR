# Resource measurements

Measured 2026-11-12 using Rust 1.99.1 release builds, thin LTO, Rune bb310fc9. Host: Ubuntu 24.04 GNOME X11, AMD Ryzen 5 5600G, approximately 15 GiB usable RAM, NVIDIA RTX 3060 / driver 595.99.02 / Vulkan. This is **not** the proposed 8 GB integrated-GPU acceptance machine.

The 150 MB resident-memory target is **not met** on this host. Timing measurements below do not certify first visible pixels; both boundaries are stated explicitly. Native Windows/macOS, physical monitor/sleep and integrated-GPU measurements remain required before a broad release.

| Measurement | Observed | Method |
|---|---:|---|
| Warm X11 shortcut to input focus, p95 | 23.97 ms | 30 invocations, XTest → XGetInputFocus, 5 ms polling |
| Warm invoke callback to next GPU frame submission, p95 | 8.22 ms | 38 warm samples, opt-in numeric-only ATHANOR_MEASURE log |
| Local 100,100-entry search, p95 | 5.12 ms | 50 measured queries after 10 warmups, alternating multiword/fuzzy queries |
| File indexing, 100,100 entries | 190.9 ms | Metadata-only core scan; fixture creation excluded |
| Core benchmark peak RSS | 103.4 MiB | Linux VmHWM; includes fixture iteration, sort/snapshot/cache/query allocations |
| Serialized cache | 42,732,027 bytes | Atomic JSON-lines write; ~222.1 ms |
| Earlier cache before compact UTF-8 encoding | 96,971,627 bytes | Native bytes remain lossless for non-UTF-8 paths |
| Final compact RSS / idle CPU | 364.3 MiB / 1.00% of one core | 5 seconds, corrected handoff release |
| Final expanded RSS / idle CPU | 364.4 MiB / 1.20% of one core | 5 seconds, corrected handoff release |
| Final hidden RSS / idle CPU | 364.4 MiB / 0.43% of one core | 60 seconds, corrected handoff release |
| Earlier compact RSS / idle CPU | 379.3 MiB / 0.37% of one core | 60 seconds, before shared-entry optimization |
| Graphics allocation | 19 MiB | nvidia-smi process row for Athanor; **separate from CPU RSS** |

Corrected-build state-by-state samples are recorded alongside the package in `artifacts/resources-*-handoff.json`; they must be read with their recorded PID/state and duration. No claim that the memory budget passed is made. The hidden CPU target is checked against a quiet minute; compact focused mode may wake for the text caret, while background filesystem events can legitimately wake the indexer.

Search partitions and published snapshots now share immutable entry allocations instead of retaining cloned path/title/target data. A proven access-notification loop was removed: indexer reads no longer schedule new scans. GPU/window state is retained to preserve quick warm invocation. The application does not hide its GPU memory cost inside the search-only benchmark.

## Reproduce

```sh
cargo run -p athanor-core --release --example benchmark -- /tmp/athanor-benchmark-files
ATHANOR_DATA_DIR=/tmp/athanor-perf ATHANOR_MEASURE=1 ./target/release/athanor
# Configure /tmp/athanor-perf/config.json search_roots to the generated fixture before launch.
python3 scripts/check-native-x11.py --leave compact
python3 scripts/measure-resources.py PID --state compact --seconds 60 --output artifacts/resources-compact-final.json
```

Use an isolated data directory; otherwise active development/user folders contaminate indexing/CPU samples. Measure hidden, compact and expanded separately after indexing settles. Record GPU memory with the platform tool. `ATHANOR_MEASURE` logs only elapsed reveal-submission durations, never queries or filenames. Native compositor/scanout latency is not included in that duration. A cold GPU/window creation can take over a second on this host; the warm target does not imply that cold launch is equally fast.
