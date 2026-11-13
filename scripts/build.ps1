param([switch]$Package)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
cargo build --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed.' }
New-Item -ItemType Directory -Force dist | Out-Null
Copy-Item target/release/codex-mini.exe dist/codex-mini.exe -Force
if ($Package) { & "$PSScriptRoot/package.ps1" }
