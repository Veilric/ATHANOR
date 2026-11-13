# Native support status

Updated 2026-11-12. Implementation and native observations are separate. `--capabilities` reports candidates and unverified checks; it does not certify the current machine.

| Platform | Implementation | Native evidence | Release gate |
|---|---|---|---|
| Linux x86_64 X11 | Rune borderless topmost window, XDG apps, XRandR work areas/active display, shortcut, tray, compositor opacity | Ubuntu 24.04 / GNOME X11 / NVIDIA RTX 3060: typing, topmost, empty pinned collapse, outside typing, Escape and 30 shortcut invocations passed | Manual IME, drag, unplug/mixed-DPI and sleep checks remain |
| Windows x86_64 | Rune window, StartApps/AppsFolder, explorer actions, monitor work areas/DPI, shortcut/tray, opt-in login registry entry | Not built/run on Windows in this session | Native build, typing/focus/topmost/transparency/shortcut, signed distribution |
| macOS arm64/x86_64 | Rune nonactivating launcher panel, native app discovery/open, macOS status menu adapter, shortcut, opt-in LaunchAgent | Not built/run on macOS in this session | Native builds, typing/focus/Spaces/transparency/shortcut and packaging/signing |
| Layer-shell Wayland | Not enabled in this preview | No compositor runner available; key/focus/resize behavior unverified | A layer-shell host and native acceptance are required before enabling |
| GNOME Wayland | Deferred; this build rejects a native Wayland session | Rune's XWayland overlay compatibility cannot take keyboard focus | Verify an explicit ordinary XWayland or future native route before support |

X11 invocation and event-driven XRandR/work-area notifications refresh monitor geometry/work areas; login1 resume notifications trigger the same recovery without requesting focus. saved offsets use logical coordinates and physical desktop origins. A preferred display wins; otherwise the foreground window's center selects the display. A pinned panel keeps its display and is clamped onto a surviving display after a notification. Physical hotplug/sleep acceptance remains unverified. Windows has work-area/foreground adapters for invocation, pending native checks and an automatic output/resume observer. macOS currently uses Rune's startup displays and explicit preferred-display fallback; live display refresh is a remaining platform gate. Displays smaller than the full view use scrolling, with the composer anchored.

The default X11 opacity mechanism uses `_NET_WM_WINDOW_OPACITY` to support NVIDIA's opaque Vulkan surfaces. It requires a compositor and dims all window content, including text. Native alpha is selectable for compatible GPUs. Changing mechanisms requires restarting. No compositor/driver changes are made automatically.

Supported OS minimum versions have **not** been certified. Development/build candidates are Ubuntu 24.04, Windows 10/11 x86_64, macOS 14+ on Apple silicon/Intel; these are test targets, not a compatibility guarantee. The pinned Rust, Rune, wgpu and tray dependencies must all pass each target's package checks. There is no claim of Linux Wayland support from a successful X11 or headless build.
