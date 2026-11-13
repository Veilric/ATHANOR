# Native acceptance checklist

Record OS/version, CPU/RAM/GPU/driver, compositor, scale/monitor layout, Rust version, Rune revision, app package SHA256, date, and each observed result. A blank/unobserved check is **unverified**, never passed. A failed essential typing/focus/topmost/shortcut check blocks support for that configuration. Linux automated evidence is `artifacts/native-x11.json`.

## Observed on GNOME X11, 2026-11-12

- [x] Native build, initially hidden resident launch and later invocation.
- [x] Shortcut focuses Tide; 30 successive warm invocations succeed.
- [x] Native key input expands compact view; nonempty query remains expanded after idle.
- [x] Empty pinned view becomes 330×53 after three seconds, without moving the input anchor.
- [x] Topmost hint survives show/focus; compositor opacity property equals 65%.
- [x] Typing in the controlled external sentinel reaches that sentinel while Tide remains pinned.
- [x] Escape yields focus and the next invocation is empty.
- [x] Local second-instance forwarding; owned-process Quit command.
- [ ] Native clipboard/open/reveal interaction across installed applications.
- [ ] Drag mark movement, saved offset after restart, preferred display selection.
- [ ] Physical monitor unplug/replug, mixed DPI, negative origins, dock/taskbar avoidance.
- [ ] Suspend/resume while visible and hidden; no invisible focused window.
- [ ] Real IME composition/candidate windows, screen-reader navigation, reduced-motion preference.
- [ ] Readability over controlled light/dark backgrounds in both native/compositor alpha modes.

Core/headless tests cover cancellation, IME state, focus policy, removed displays, negative origins and mixed DPI; those do not replace the physical checks above. Native measurement logs do not contain queries, filenames, or clipboard content.

## Windows and macOS

Run scripts/check-native.ps1 (Windows) or check-native.sh (macOS), then perform the same native checklist manually on each supported architecture. Check Option+Space/Ctrl+Alt+Space conflicts; a failed replacement must leave the old shortcut working. Verify menu/tray Show, Settings and Quit, fresh hidden startup, Escape while pinned, action failure preserving input, opening another app yielding focus, and start-at-login only after opting in. macOS also needs Spaces/fullscreen, nonactivating panel and menu-bar checks. Windows needs desktop scaling, AppsFolder launch and file-association checks.

Neither platform was built or run here. Local workflow definitions are prepared but have not been pushed or executed. Packages remain unsigned. OS minimum versions remain a certification gate.

## Wayland

Native Wayland is disabled in this preview. A future layer-shell host must pass keyboard-on-demand, outside focus, global portal shortcut, resizing/drag placement, and output-removal checks on each enabled compositor. GNOME requires a separate verified compatibility route; an overlay that cannot type is not acceptable. The current error directs users to a real X11 session without silently changing their session or environment.
