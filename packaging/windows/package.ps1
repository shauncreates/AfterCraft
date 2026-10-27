# Compatibility entry point: packages AfterCraft with its independent Inno installer.
param(
    [ValidateSet('x64')][string]$Arch='x64',
    [switch]$SkipBuild,
    [string]$Python='python',
    [string]$IsccPath='ISCC.exe',
    [string]$RceditPath=''
)
$ErrorActionPreference='Stop'
$root=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
if(!$SkipBuild) {& (Join-Path $root 'build.ps1') -Configuration Release -Python $Python -RceditPath $RceditPath}
& (Join-Path $PSScriptRoot 'build-installer.ps1') -IsccPath $IsccPath
