$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
cargo test --workspace --locked
if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
cargo build --release --locked -p athanor --bin athanor
if ($LASTEXITCODE -ne 0) { throw 'Native build failed' }
Write-Host 'Build/tests passed. Complete docs/verification/native-checklist.md; typing/focus/topmost/shortcut remain unverified until observed.'
