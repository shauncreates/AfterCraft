[CmdletBinding(SupportsShouldProcess=$true)]
param(
    [ValidateSet('EffectCraft','AfterCraft','AfterEffects')][string]$HostApp='EffectCraft',
    [ValidateSet('Docked','Floating','Both')][string]$Mode='Both',
    [string]$SettingsFolder=''
)
$ErrorActionPreference='Stop'
$source=Join-Path $PSScriptRoot 'Current.jsx'
if(!(Test-Path -LiteralPath $source -PathType Leaf)) {$source=Join-Path $PSScriptRoot '../scriptui-panels/Current.jsx'}
$source=(Resolve-Path -LiteralPath $source).Path
if(!$SettingsFolder) {
    if($HostApp -eq 'AfterEffects') {throw 'Use After Effects File > Scripts > Install ScriptUI Panel, or supply its existing versioned per-user preferences folder with -SettingsFolder.'}
    $SettingsFolder=Join-Path ([Environment]::GetFolderPath('ApplicationData')) $HostApp
}
$destinationRoot=[IO.Path]::GetFullPath($SettingsFolder)
if($HostApp -eq 'AfterEffects' -and !(Test-Path -LiteralPath $destinationRoot -PathType Container)) {throw 'The After Effects preferences folder must already exist; choose the correct installed version.'}
foreach($restricted in @(${env:ProgramFiles},${env:ProgramFiles(x86)},$env:windir)) {
    if($restricted) {
        $restrictedRoot=[IO.Path]::GetFullPath($restricted).TrimEnd('\')
        if($destinationRoot.Equals($restrictedRoot,[StringComparison]::OrdinalIgnoreCase) -or $destinationRoot.StartsWith($restrictedRoot+'\',[StringComparison]::OrdinalIgnoreCase)) {
            throw 'Current uses a per-user script folder. This installer does not write into application or Windows installation directories.'
        }
    }
}
$files=@()
if($Mode -in @('Docked','Both')) {$files+=Join-Path $destinationRoot 'Scripts\ScriptUI Panels\Current.jsx'}
if($Mode -in @('Floating','Both')) {$files+=Join-Path $destinationRoot 'Scripts\Current Floating.jsx'}
foreach($path in $files) {
    if($PSCmdlet.ShouldProcess($path,'Install Current portable ScriptUI extension')) {
        $parent=Split-Path -Parent $path
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
        if(Test-Path -LiteralPath $path) {Copy-Item -LiteralPath $path -Destination ($path+'.backup-'+[DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ'))}
        Copy-Item -LiteralPath $source -Destination $path
        [PSCustomObject]@{Host=$HostApp;Mode=$Mode;Installed=$path;SHA256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash}
    }
}
Write-Host 'Docked: Window > Current.jsx. Floating: File > Scripts > Current Floating.jsx, or Run Script File > Current.jsx. Restart After Effects if its menus have not refreshed.'
