param([switch]$Package, [switch]$Gpu, [string]$Target = 'x86_64-pc-windows-msvc')
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
if ($Target -ne 'x86_64-pc-windows-msvc') { throw 'Windows packaging supports x86_64-pc-windows-msvc.' }
$buildArgs = @('build', '--release', '--locked', '--target', $Target)
if ($Gpu) { $buildArgs += @('--features', 'gpu') }
$previousEncodedFlags = $env:CARGO_ENCODED_RUSTFLAGS
$releaseFlags = @()
if ($previousEncodedFlags) { $releaseFlags += $previousEncodedFlags.Split([char]31) }
elseif ($env:RUSTFLAGS) { $releaseFlags += ($env:RUSTFLAGS -split '\s+' | Where-Object { $_ }) }
$releaseFlags += @('-C', 'target-feature=+crt-static')
$taskUserProfile = [Environment]::GetFolderPath('UserProfile')
$remapRoots = @(@($taskUserProfile, '/build/home'), @($PWD.Path, '/build/codex-mini'))
if ($env:CARGO_HOME) { $remapRoots += ,@($env:CARGO_HOME, '/build/cargo') }
if ($env:RUSTUP_HOME) { $remapRoots += ,@($env:RUSTUP_HOME, '/build/rustup') }
foreach ($entry in $remapRoots) {
    if ($entry[0]) {
        $releaseFlags += "--remap-path-prefix=$($entry[0])=$($entry[1])"
        $releaseFlags += "--remap-path-prefix=$($entry[0].Replace('\', '/'))=$($entry[1])"
    }
}
try {
    $env:CARGO_ENCODED_RUSTFLAGS = $releaseFlags -join [char]31
    cargo @buildArgs
    if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed.' }
} finally {
    if ($null -eq $previousEncodedFlags) { Remove-Item Env:CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue }
    else { $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedFlags }
}
New-Item -ItemType Directory -Force dist | Out-Null
try {
    Copy-Item "target/$Target/release/codex-mini.exe" dist/codex-mini.exe -Force -ErrorAction Stop
} catch [IO.IOException] {
    # Windows permits renaming a running image; leave its process undisturbed.
    $previousImage = "dist/codex-mini-running-$([Guid]::NewGuid().ToString('N')).exe"
    Move-Item -LiteralPath (Join-Path $PWD 'dist/codex-mini.exe') -Destination (Join-Path $PWD $previousImage)
    Copy-Item "target/$Target/release/codex-mini.exe" dist/codex-mini.exe -Force
}
if ($Package) { & "$PSScriptRoot/package.ps1" }
