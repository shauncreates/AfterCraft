param(
    [ValidateSet('Debug','Release')][string]$Configuration='Debug',
    [string]$Output=(Join-Path $PSScriptRoot 'artifacts\AfterCraft'),
    [string]$Python='python',
    [string]$PythonArchive='',
    [string]$RceditPath='',
    [switch]$SkipNative,
    [switch]$SkipRuntime,
    [int]$Jobs=2
)
$ErrorActionPreference='Stop'
if(!$env:CARGO_PROFILE_DEV_DEBUG) {$env:CARGO_PROFILE_DEV_DEBUG='0'}
. (Join-Path $PSScriptRoot 'tools\windows-toolchain.ps1')
$cargo=Initialize-AfterCraftToolchain
$profile=if($Configuration -eq 'Release') {'release'} else {'debug'}
$flags=if($Configuration -eq 'Release') {@('--release')} else {@()}
$coreTarget=if($env:CARGO_TARGET_DIR) {[IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)} else {Join-Path $PSScriptRoot 'target'}
New-Item -ItemType Directory -Force -Path $Output | Out-Null
Push-Location $PSScriptRoot
try {
    & $cargo build --locked -j $Jobs -p effectcraft -p effectcraft-cli @flags
    if($LASTEXITCODE -ne 0) {throw 'AfterCraft core build failed'}
    Copy-Item -LiteralPath (Join-Path $coreTarget "$profile\effectcraft.exe") -Destination (Join-Path $Output 'AfterCraft.exe')
    Copy-Item -LiteralPath (Join-Path $coreTarget "$profile\effectcraft-cli.exe") -Destination (Join-Path $Output 'aftercraft-cli.exe')
    if(!$SkipNative) {
        foreach($helper in @(@('native-host','effectcraft-aex-probe'),@('ofx-helper','effectcraft-helper-ofx'))) {
            $helperRoot=Join-Path $PSScriptRoot ('native\'+$helper[0])
            $helperTarget=Join-Path $helperRoot 'target'
            $env:CARGO_TARGET_DIR=$helperTarget
            Push-Location $helperRoot
            try {& $cargo build --locked -j $Jobs -p $helper[1] @flags; if($LASTEXITCODE -ne 0) {throw ($helper[0]+' build failed')}} finally {Pop-Location}
            Copy-Item -LiteralPath (Join-Path $helperTarget ($profile+'\'+$helper[1]+'.exe')) -Destination $Output
        }
    }
    $converter=Join-Path $Output 'aep-import'
    New-Item -ItemType Directory -Force -Path $converter | Out-Null
    Get-ChildItem (Join-Path $PSScriptRoot 'tools\aep-import') -File | Copy-Item -Destination $converter
    if(!$SkipRuntime) {& (Join-Path $PSScriptRoot 'packaging\python\prepare-runtime.ps1') -Destination (Join-Path $Output 'runtime\python') -Python $Python -Archive $PythonArchive}
    $licenses=Join-Path $Output 'licenses'
    New-Item -ItemType Directory -Force -Path $licenses | Out-Null
    foreach($name in @('LICENSE-MIT','LICENSE-APACHE','NOTICE','PROVENANCE.md','ATTRIBUTION.md')) {Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination $licenses}
    foreach($helper in @('native-host','ofx-helper')) {
        $destination=Join-Path $licenses $helper
        New-Item -ItemType Directory -Force -Path $destination | Out-Null
        Get-ChildItem (Join-Path $PSScriptRoot ('native\'+$helper)) -File | Where-Object {$_.Name -like 'LICENSE*' -or $_.Name -in @('ATTRIBUTION.md','PROVENANCE.md')} | Copy-Item -Destination $destination
    }
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'README.md') -Destination $Output
    $fonts=Join-Path $licenses 'fonts'
    New-Item -ItemType Directory -Force -Path $fonts | Out-Null
    Get-ChildItem (Join-Path $PSScriptRoot 'assets/fonts') -Filter '*.txt' | Copy-Item -Destination $fonts
    & $Python (Join-Path $PSScriptRoot 'tools/collect-licenses.py') (Join-Path $licenses 'rust-dependencies')
    if($LASTEXITCODE -ne 0) {throw 'Dependency license collection failed'}
    if($RceditPath) {
        & $RceditPath (Join-Path $Output 'AfterCraft.exe') --set-icon (Join-Path $PSScriptRoot 'assets\aftercraft\aftercraft.ico') --set-version-string ProductName AfterCraft --set-version-string FileDescription 'AfterCraft motion graphics and VFX' --set-version-string OriginalFilename AfterCraft.exe --set-file-version 0.16.0.1 --set-product-version 0.16.0.1
        if($LASTEXITCODE -ne 0) {throw 'AfterCraft executable resources failed'}
    }
    Get-ChildItem -LiteralPath $Output -File -Filter '*.exe' | ForEach-Object {
        [PSCustomObject]@{File=$_.Name; SHA256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash}
    } | ConvertTo-Json | Set-Content (Join-Path $Output 'SHA256.json')
} finally {$env:CARGO_TARGET_DIR=$coreTarget; Pop-Location}
