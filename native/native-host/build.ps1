param([ValidateSet('build','test','check','fmt','clippy')][string]$Action='test')
$ErrorActionPreference='Stop'
$root=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
. (Join-Path $root 'tools/windows-toolchain.ps1')
$cargo=Initialize-AfterCraftToolchain
$previousTarget=$env:CARGO_TARGET_DIR
$env:CARGO_TARGET_DIR=Join-Path $PSScriptRoot 'target'
Push-Location $PSScriptRoot
try {
    if($Action -eq 'fmt') {& $cargo fmt --all -- --check}
    elseif($Action -eq 'clippy') {& $cargo clippy --locked --workspace --all-targets -- -D warnings}
    else {& $cargo $Action --locked --workspace -j 2}
    if($LASTEXITCODE -ne 0) {throw "Native $Action failed"}
} finally {$env:CARGO_TARGET_DIR=$previousTarget; Pop-Location}
