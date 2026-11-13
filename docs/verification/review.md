# Independent review and corrections

An independent read-only reviewer inspected the whole local source against the approved design/plan. There is no usable Git commit range. The original source manifest and execution logs are retained under `.superpowers/sdd/2026-11-12-athanor-implementation/`.

No Critical findings; six Important findings were addressed in one regression-tested correction pass:

| Finding | Correction and evidence |
|---|---|
| Relative cache paths caused self-triggered rescans | Absolute, parent-canonicalized cache destination; `relative_cache_inside_an_indexed_root_does_not_rescan_itself` failed with repeated revisions, then passed |
| Tiny inputs/conversions silently became zero | Exact Decimal parsing and nonzero-to-zero range guards in measurement/rate conversion; parse and multiplication underflow tests failed first |
| Scaled bits became bytes | Case-sensitive decimal/binary storage symbols; `kb`, `Mb`, `Kib`, `kB` regression assertions failed first |
| Opener exit failures dismissed input | Bounded background action worker; helper exit confirmation/timeout; disappearing application checks; Windows ShellExecute error results; helper/UI completion tests failed first |
| Display/resume recovery had no event source | Linux XRandR/work-area and login1 wake notifications now trigger geometry recovery without focus; reducer and live X11 synthetic-notification tests failed first. Windows/macOS observers remain an explicit release gate |
| Secondary text lacked light-background contrast | Brighter secondary token; actual Rune token tested against light/dark backdrops using both alpha mechanisms, failing first at 3.43:1 and then passing 4.5:1 |

Additional regressions during this pass covered action completion ordering and listener shutdown. The X11 sender now round-trips before disconnecting; otherwise an unprocessed notification/stop message could disappear and leave shutdown waiting. The native notification/shutdown test passes in approximately 0.01 seconds without changing the physical display layout.

The exact currency amount error path also explains malformed/comma amounts, resolving the review's fourth minor finding as a consequence of the required precision fix.

## Deferred minor findings

1. Input activity that does not edit text (pointer/key navigation, beginning a drag) does not reset the empty pinned idle deadline. Edits, pin/focus transitions and settings already update/suspend it.
2. Calculator classification misses unsigned scientific exponents such as `1e9 + 1`; use `1e+9 + 1` or `1000000000 + 1`. Signed exponents work.
3. Reset placement while pinned can record the old placement again. Unpin before resetting.

## Limits retained after review

Windows/macOS native build/package behavior, Wayland hosting/compatibility, physical IME/accessibility/mixed-DPI/sleep, the integrated-GPU resource budget, signing and publication are not certified by this review. Source-only Windows/macOS automatic output/resume observers remain implementation work. Local package contents and final measurements are checked by the implementer and documented separately. The 150 MB resident-memory target is unmet on the measured NVIDIA host.
