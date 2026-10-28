param(
    [string]$Payload = (Join-Path $PSScriptRoot '..\..\artifacts\AfterCraft'),
    [string]$IsccPath = 'ISCC.exe',
    [string]$Version = '0.16.0.2'
)
$ErrorActionPreference = 'Stop'
$payloadPath = (Resolve-Path -LiteralPath $Payload).Path
if (!(Test-Path -LiteralPath (Join-Path $payloadPath 'AfterCraft.exe'))) {throw 'Build the AfterCraft payload first'}
$privateFiles=@(Get-ChildItem -LiteralPath $payloadPath -File -Recurse | Where-Object {
    $_.Extension -in @('.aep','.aepx','.aex','.ofx','.ecproj','.ecprojx') -or
    $_.Name -in @('session.lock','prefs.json','native-plugin-preference.json')
})
if($privateFiles.Count -gt 0) {throw 'Installer payload contains private projects, vendor plugins or user settings'}
foreach($required in @('aftercraft-cli.exe','effectcraft-aex-probe.exe','effectcraft-helper-ofx.exe','runtime/python/python.exe','aep-import/aep2effectcraft.py','licenses/rust-dependencies/inventory.json')) {
    if(!(Test-Path -LiteralPath (Join-Path $payloadPath $required))) {throw "Incomplete payload: $required"}
}
& $IsccPath (Join-Path $PSScriptRoot 'aftercraft.iss') ('/DAppVersion='+$Version) ('/DPayload='+$payloadPath)
if ($LASTEXITCODE -ne 0) {throw 'AfterCraft installer compilation failed'}
