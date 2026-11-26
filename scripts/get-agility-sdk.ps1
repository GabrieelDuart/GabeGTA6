# Puts Microsoft's DirectX 12 Agility SDK 1.614 beside victory.exe (target\<build>\D3D12),
# which Windows 10 needs to run the title's root signatures (docs/gtx1060.md).
#   scripts\get-agility-sdk.ps1            (target\diag and target\release)
#   scripts\get-agility-sdk.ps1 -Build diag
# The DLLs come from the official NuGet package Microsoft.Direct3D.D3D12 and are never
# committed (docs/asset-policy.md). victory.exe asks for SDK version 614 (apps/victory/src/main.rs).
param(
    [ValidateSet('diag', 'release', 'both')][string]$Build = 'both',
    [string]$Version = '1.614.1'
)
$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot '..'
# Downloaded beside the build (target\agility), on the project's own disk, not in %TEMP%.
$work = Join-Path $root "target\agility\$Version"
New-Item -ItemType Directory -Force -Path (Split-Path $work) | Out-Null
$zip = "$work.zip"
if (-not (Test-Path (Join-Path $work 'build\native\bin\x64\D3D12Core.dll'))) {
    $url = "https://api.nuget.org/v3-flatcontainer/microsoft.direct3d.d3d12/$Version/microsoft.direct3d.d3d12.$Version.nupkg"
    Write-Host "Downloading $url"
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
    if (Test-Path $work) { Remove-Item -Recurse -Force $work }
    Expand-Archive -Path $zip -DestinationPath $work
}
$bin = Join-Path $work 'build\native\bin\x64'
$builds = if ($Build -eq 'both') { 'diag', 'release' } else { , $Build }
foreach ($b in $builds) {
    $dest = Join-Path $root "target\$b\D3D12"
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    Copy-Item (Join-Path $bin 'D3D12Core.dll'), (Join-Path $bin 'd3d12SDKLayers.dll') $dest -Force
    Write-Host "Agility SDK $Version in $dest"
}
