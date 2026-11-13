#!/usr/bin/env python3
"""Run Cargo using workspace-local caches/SDK without modifying the host."""
from pathlib import Path
import os
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
env = os.environ.copy()
env.setdefault("CARGO_HOME", str(root / ".local-run/cargo"))
env.setdefault("ATHANOR_DATA_DIR", str(root / ".local-run/data"))
local_tools = root / ".local-run/rust-tools/bin"
if local_tools.exists():
    env["PATH"] = str(local_tools) + os.pathsep + env.get("PATH", "")
    rust_lib = Path.home() / ".rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib"
    env["LD_LIBRARY_PATH"] = str(rust_lib) + os.pathsep + env.get("LD_LIBRARY_PATH", "")
if "RUSTUP_TOOLCHAIN" not in env:
    stable = Path.home() / ".rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc"
    named = Path.home() / ".rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu"
    if stable.exists() and not named.exists():
        version = subprocess.check_output([str(stable), "--version"], text=True)
        if version.startswith("rustc 1.99.1 "):
            env["RUSTUP_TOOLCHAIN"] = "stable"
sdk = root / ".local-run/sdk"
if sdk.exists():
    pc = sdk / "usr/lib/x86_64-linux-gnu/pkgconfig"
    env.setdefault("PKG_CONFIG_SYSROOT_DIR", str(sdk))
    env["PKG_CONFIG_PATH"] = str(pc) + os.pathsep + env.get("PKG_CONFIG_PATH", "")
    libs = str(sdk / "usr/lib/x86_64-linux-gnu")
    env["LIBRARY_PATH"] = libs + os.pathsep + env.get("LIBRARY_PATH", "")
    env["LD_LIBRARY_PATH"] = libs + os.pathsep + env.get("LD_LIBRARY_PATH", "")
args = sys.argv[1:]
if args and args[0] == "--portable":
    args.pop(0)
    env["CARGO_TARGET_DIR"] = str(root / ".local-run/portable-target")
raise SystemExit(subprocess.call(["cargo", *args], cwd=root, env=env))
