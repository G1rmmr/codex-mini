$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
$version = (Select-String '^version = "([^"]+)"' Cargo.toml | Select-Object -First 1).Matches.Groups[1].Value
$name = "codex-mini-$version-windows-x86_64"
$staging = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
$package = Join-Path $staging $name
New-Item -ItemType Directory -Path $package -Force | Out-Null
try {
    Copy-Item dist/codex-mini.exe,dist/THIRD_PARTY_LICENSES.html,LICENSE,README.md,THIRD_PARTY_NOTICES.md $package
    Copy-Item licenses,docs $package -Recurse
    Compress-Archive -Path $package -DestinationPath "dist/$name.zip" -Force
} finally {
    Remove-Item $staging -Recurse -Force
}
