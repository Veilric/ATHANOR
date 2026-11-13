param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Set-Location (Split-Path $PSScriptRoot -Parent)
$target = 'x86_64-pc-windows-msvc'
$env:CARGO_BUILD_JOBS = '2'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
if ($env:OS -ne 'Windows_NT') { throw 'Package on native Windows' }
if (-not $SkipBuild) {
    cargo build --release --locked --jobs 2 --target $target -p athanor --bin athanor
    if ($LASTEXITCODE -ne 0) { throw 'Native build failed' }
}
$stage = Join-Path 'artifacts' 'windows-x64-portable'
$archive = Join-Path 'artifacts' 'ATHANOR-windows-x64-portable.zip'
if ((Test-Path $stage) -or (Test-Path $archive)) { throw 'Existing package output must be moved aside before packaging again' }
$source = (git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Source commit is required' }
$changes = git status --porcelain --untracked-files=normal
if ($LASTEXITCODE -ne 0 -or $changes) { throw 'Package requires a clean committed source tree' }
New-Item -ItemType Directory -Path $stage | Out-Null
Copy-Item "target/$target/release/athanor.exe" (Join-Path $stage 'ATHANOR.exe')
Copy-Item 'docs/windows-portable.md' (Join-Path $stage 'WINDOWS-USAGE.md')
Copy-Item -Recurse assets (Join-Path $stage 'assets')
cargo metadata --locked --format-version 1 | Set-Content -Encoding utf8 'artifacts/windows-metadata.json'
if ($LASTEXITCODE -ne 0) { throw 'Dependency metadata failed' }
python scripts/dependency-notices.py (Join-Path $stage 'licenses') --metadata artifacts/windows-metadata.json
if ($LASTEXITCODE -ne 0) { throw 'Dependency notices failed' }
$compiler = (rustc --version).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Compiler version unavailable' }
$info = [ordered]@{
    product = 'ATHANOR - by Veilric'
    version = '0.1.0-preview.1'
    source_commit = $source
    target = $target
    compiler = $compiler
    rune_revision = 'bb310fc9e2c4a1a1937814c71efc08924a5c2885'
    built_utc = [DateTime]::UtcNow.ToString('o')
    static_crt = $true
    signed = $false
    windows_gui_runtime_verified = $false
}
$info | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $stage 'BUILD_INFO.json')
Compress-Archive -Path "$stage/*" -DestinationPath $archive
python scripts/verify-windows-package.py $archive --smoke --source-commit $source --report artifacts/windows-verification.json
if ($LASTEXITCODE -ne 0) { throw 'Portable package verification failed' }
$hash = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
"$hash  ATHANOR-windows-x64-portable.zip" | Set-Content -Encoding ascii "$archive.sha256"
Write-Host "$archive ($((Get-Item $archive).Length) bytes, SHA256 $hash)"
