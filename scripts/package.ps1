$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
cargo build --release --locked -p athanor --bin athanor
if ($LASTEXITCODE -ne 0) { throw 'Native build failed' }
$label = 'athanor-0.1.0-Windows-x86_64'
$stage = Join-Path 'artifacts' $label
if (Test-Path $stage) { throw "Move existing staging folder aside: $stage" }
New-Item -ItemType Directory -Path $stage | Out-Null
Copy-Item target/release/athanor.exe $stage
Copy-Item README.md $stage
Copy-Item -Recurse docs (Join-Path $stage 'docs')
python scripts/dependency-notices.py (Join-Path $stage 'licenses')
if ($LASTEXITCODE -ne 0) { throw 'Dependency notices failed' }
'Version: 0.1.0; Rune: bb310fc9e2c4a1a1937814c71efc08924a5c2885; Unsigned local preview' | Set-Content (Join-Path $stage 'BUILD_INFO.txt')
Compress-Archive -Path $stage -DestinationPath "artifacts/$label.zip"
Write-Host "artifacts/$label.zip"
