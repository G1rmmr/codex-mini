$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
$version = (Select-String '^version = "([^"]+)"' Cargo.toml | Select-Object -First 1).Matches.Groups[1].Value
$name = "codex-mini-$version-windows-x86_64"
$stagingRoot = [IO.Path]::GetFullPath((Join-Path $PWD 'target/package-staging'))
$staging = [IO.Path]::GetFullPath((Join-Path $stagingRoot ([Guid]::NewGuid().ToString())))
$package = Join-Path $staging $name
New-Item -ItemType Directory -Path $package -Force | Out-Null
try {
    Copy-Item dist/codex-mini.exe,dist/THIRD_PARTY_LICENSES.html,LICENSE,README.md,THIRD_PARTY_NOTICES.md $package
    Copy-Item licenses,docs $package -Recurse
    Compress-Archive -Path $package -DestinationPath "dist/$name.zip" -Force
} finally {
    $resolvedStaging = [IO.Path]::GetFullPath($staging)
    if (-not $resolvedStaging.StartsWith($stagingRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to remove a staging path outside target/package-staging.'
    }
    Remove-Item -LiteralPath $resolvedStaging -Recurse -Force
}
