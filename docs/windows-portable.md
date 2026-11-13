# ATHANOR — by Veilric

## Windows portable preview

This unsigned x64 preview requires Windows 10 or Windows 11, a compatible graphics driver, and an interactive desktop. Windows GUI and runtime acceptance on a separate PC remains pending. CI checks compilation, automated tests, archive structure, PE architecture, and isolated non-GUI queries.

Download `ATHANOR-windows-x64-portable.zip` and its `.sha256` file from the private repository's preview release while signed in to an authorized GitHub account. Extract the entire ZIP to a user-writable folder. Keep the executable, assets, notices, and build information together. Rust, Cargo, Codex, and a development environment are unnecessary on the test PC. The executable includes the C runtime statically. Windows supplies system libraries and the Direct3D FXC shader compiler used by the graphics backend's fallback.

The package is unsigned. Windows, SmartScreen, antivirus, or company policy may warn or block it. On a work PC, obtain IT approval and follow the organization's software rules. Record any warning and stop testing if policy prevents execution.

Verify the ZIP in PowerShell before extracting it. Compare the hash against the first field in the downloaded checksum file and the release notes.

```powershell
Get-FileHash .\ATHANOR-windows-x64-portable.zip -Algorithm SHA256
```

Open PowerShell in the extracted folder and launch the panel.

```powershell
.\ATHANOR.exe --show
```

Use **Ctrl + Alt + Space** to summon ATHANOR. The tray/menu offers Show, Settings, and Quit. Escape clears and hides the panel. Starting another instance summons the existing one. This command quits a running instance.

```powershell
.\ATHANOR.exe --quit
```

Settings, search metadata, and cached rates use the existing user-profile location `%APPDATA%\Veilric\Athanor\config`. Moving or replacing the extracted package preserves that data. An explicitly supplied `ATHANOR_DATA_DIR` continues to override the location. Start at login is off by default. If enabled, it refers to the current executable path, so update that setting after moving the package.

`BUILD_INFO.json` records the source SHA, compiler, target, Rune revision, and preview status. Third-party metadata and license texts are in `licenses`. No public license has been assigned to ATHANOR's own source.

## Windows acceptance checklist

Record Windows version, GPU/driver, display scaling, package SHA256, and each result. These checks require the actual test PC. A completed CI run leaves this checklist pending.

- [ ] Launch `ATHANOR.exe --show` from the extracted folder. Confirm the translucent Tide panel appears and accepts typing.
- [ ] Press Ctrl + Alt + Space from another application. Confirm reliable summoning without duplicate instances. Check shortcut conflicts in Settings.
- [ ] Evaluate `sqrt(144)+2^5` and confirm `44`. Try the calculator angle setting.
- [ ] Convert `32 F to C` and confirm `0 °C`. Try `12 km in mi` and `1 MiB to KiB`.
- [ ] Evaluate `2026-11-11 + 45 days` and confirm `Dec 26, 2026`. Try `today - 7 days`.
- [ ] Convert `100 usd to php` with a permitted network connection. Confirm provider/date feedback. Repeat offline after caching and inspect offline/stale labels.
- [ ] Open `help`, `help calculator`, `help units`, `help calendar`, `help currency`, and `help search`. Scroll the references and try their examples. Confirm an example is staged until Enter.
- [ ] Search for an installed application and explicitly launch it. Confirm the panel stays open after a successful action.
- [ ] Search for known files and folders in Documents, Desktop, Downloads, or configured roots. Test Open and Reveal and record discovery omissions.
- [ ] Copy a calculation result and a file path, then paste each into a text editor.
- [ ] Unpin and switch to another app. Confirm ATHANOR stays visible at normal window level and can be covered by another window.
- [ ] Pin and switch apps. Confirm ATHANOR stays above other windows. With an empty query, wait three seconds and confirm only the chatbox and unpin control remain. Type to expand.
- [ ] Drag the leading mark in both modes. Verify position across hide/summon, display changes, scaling, and restart. Escape should clear and hide in either mode.
- [ ] Open Settings. Change opacity, shortcut, search roots, and angle mode. Restart and confirm persistence. Test placement reset. Test start at login only if workplace policy permits.
- [ ] Close/hide and resummon. Quit through the tray/menu, then repeat with `ATHANOR.exe --quit`. Confirm the process exits and the shortcut is released.

Record launch errors, missing DLL messages, security warnings, rendering issues, application discovery gaps, and failing checklist steps. The first currency conversion needs an available provider when rates have never been cached. Windows accessibility, IME, multiple desktops, and resource usage also require native acceptance. Linux's measured memory budget remains unmet in the existing verification record.
