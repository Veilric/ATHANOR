# ATHANOR — by Veilric

## Sprint 01 verification

The sprint changes visibility, pinning and command discovery. The development executable is `target/debug/athanor`. Run it with `--show`, or summon the resident application with Ctrl+Alt+Space on Linux and Windows. Escape explicitly hides and clears the query. The tray menu provides Quit.

### Command reference

Enter `help`, `help calculator`, `help units`, `help calendar`, `help currency` or `help search` in the search input. Choose a category to open its reference. Choose an example to load an editable draft. Enter evaluates the draft. The next Enter uses the selected result action. Up and Down retain result navigation. Help content is bundled with the application and works offline.

### Automated checks

The requested format check, workspace tests and development build passed with two Cargo jobs. The workspace suite reports 74 passed and three opt-in tests skipped. Clippy with warnings denied also passed. An interim format check caught spacing in a newly added assertion, which was corrected before the passing final run.

The controlled X11 run observed normal unpinned level, actual dragging in both modes, outside-focus persistence, retained origins, pin-level changes, native clipboard copying, explicit close/resummon, normal Quit and 30 summon cycles. Physical workspace switching and real-application occlusion remain manual checks.

The sprint has regression coverage for focus loss, pin-level effects, query preservation, explicit hiding, drag-handle clicks in the Rune runtime, help precedence, advertised examples, cancellation and staged input. The staged-input test checks editing, composition and service-driven refresh before Enter.

The requested verification commands use the existing local environment helper in this checkout.

```sh
python3 scripts/cargo-local.py fmt --all -- --check
python3 scripts/cargo-local.py test --workspace --locked -j 2
python3 scripts/cargo-local.py build --locked -p athanor --bin athanor -j 2
```

### Manual Linux X11 checklist

Use the development executable and record the session, compositor and result for each observation.

- [ ] Unpinned, click the drag handle once. The window stays visible and the query remains intact.
- [ ] Unpinned, drag the handle to a different position. The window follows the pointer and accepts typing afterward.
- [ ] Unpinned, click another application. ATHANOR stays open at normal window level and can appear behind that application.
- [ ] Summon the already-visible window. Focus returns to the input and its screen origin stays unchanged.
- [ ] Pin the window with a nonempty query. The query and screen origin stay unchanged and the window becomes topmost.
- [ ] Click outside and switch applications while pinned. ATHANOR stays visible above them.
- [ ] Drag while pinned. The selected origin is retained across focus changes.
- [ ] Move the owned window between workspaces and return. The origin and pin state stay unchanged.
- [ ] Unpin. The window returns to normal level and retains its query and origin.
- [ ] Copy or open a successful result while unpinned. The window stays open.
- [ ] Use Escape and the summon shortcut. Explicit hiding clears the query and summoning still works.
- [ ] Close the owned window, summon it again, then use tray Quit. Explicit close and quit remain effective.
- [ ] Open every help category with the network disconnected. Syntax, examples and limitations remain readable and scrollable.
- [ ] Select an example with pointer and keyboard. Edit the draft before Enter and confirm evaluation starts only after submission.
- [ ] Confirm `1+1`, `sqrt(144)+2^5`, `12 km in mi`, `today + 30 days` and `2022-01-15 + 180 days` still produce results.
- [ ] With a connection, verify `25 usd to php` and `100 eur to usd`. With cached data, disconnect and confirm the dated cached reference remains available.

The controlled X11 script covers a subset with its own sentinel window. Run it against a fresh, unpinned development instance.

```sh
python3 scripts/check-native-x11.py --output /tmp/athanor-sprint-01-x11.json
```

Physical workspace changes, real application occlusion, close controls and network transitions require the manual observations above. Windows native acceptance remains pending until the sprint is tested on Windows.

### Git state

At sprint start, both `git status` and `git remote -v` report that this checkout has no usable repository metadata. Commit hashes and an existing push destination are unavailable. A focused source snapshot and diff are retained in `.superpowers/sdd/2026-11-14-sprint-01` for review.

### Modified files

Core behavior and help data changed in these files.

- `crates/core/src/panel.rs`
- `crates/core/src/feature.rs`
- `crates/core/src/query.rs`
- `crates/core/src/features/mod.rs`
- `crates/core/src/features/help.rs` was added.

Native hosting and Tide integration changed in these files.

- `crates/app/src/bootstrap.rs`
- `crates/app/src/ui/mod.rs`
- `crates/app/src/ui/native.rs`
- `crates/app/src/ui/tide.rs`
- `crates/app/src/ui/scenes.rs`

Verification changed in these files.

- `crates/core/tests/panel.rs`
- `crates/core/tests/feature_contract.rs`
- `crates/core/tests/help.rs` was added.
- `crates/app/tests/ui.rs`
- `crates/app/tests/cli.rs`
- `scripts/check-native-x11.py`

The sprint plan and this verification record were added under `docs/superpowers/plans/2026-11-14-sprint-01.md` and `docs/verification/sprint-01.md`. The existing calculator, measurements, calendar, currency, search parser and rate-service source files match the pre-sprint snapshot.

### Final review

The independent source review found no Critical or Important findings. The pin label and query-triggered currency retry wording were clarified after review. Help root, calculator and currency pages were rendered and visually inspected. Native close/resummon passed after the test waited for WM withdrawal and moved the pointer before changing fixture focus. The test restores the original pointer and prior external focus, and its owned ATHANOR process exits through Quit.
