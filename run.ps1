$ErrorActionPreference = 'Stop'
$binary = Join-Path $PSScriptRoot 'dist/codex-mini.exe'
if (-not (Test-Path $binary)) {
    $binary = Join-Path $PSScriptRoot 'target/release/codex-mini.exe'
}
if (-not (Test-Path $binary)) { throw 'Build first: .\scripts\build.ps1' }
& $binary @args | Out-Host
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
