param([Parameter(Mandatory=$true)][string]$Installer)
$ErrorActionPreference='Stop'
$installerPath=(Resolve-Path -LiteralPath $Installer).Path
$uninstallKey='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{81C0B7DA-6755-4E89-9655-CC91BC49117A}_is1'
$appPathKey='HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths\AfterCraft.exe'
$startLink=Join-Path ([Environment]::GetFolderPath('Programs')) 'AfterCraft\AfterCraft.lnk'
$desktopLink=Join-Path ([Environment]::GetFolderPath('DesktopDirectory')) 'AfterCraft.lnk'
if((Test-Path -LiteralPath $uninstallKey) -or (Test-Path -LiteralPath $appPathKey) -or (Test-Path -LiteralPath $startLink) -or (Test-Path -LiteralPath $desktopLink)) {
    throw 'Run installer smoke tests in a clean environment without an existing AfterCraft installation or shortcut.'
}
$work=Join-Path ([IO.Path]::GetTempPath()) ('AfterCraft installer test '+[Guid]::NewGuid().ToString('N'))
$target=Join-Path $work 'AfterCraft app'
New-Item -ItemType Directory -Force -Path $work | Out-Null
$originals=@('C:\Program Files\EffectCraft\effectcraft.exe','C:\Program Files\EffectCraft\effectcraft-cli.exe') | Where-Object {Test-Path -LiteralPath $_}
$before=@{}
foreach($file in $originals) {$before[$file]=(Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash}
$oldConfig=$env:AFTERCRAFT_CONFIG_DIR
$oldBytecode=$env:PYTHONDONTWRITEBYTECODE
$env:AFTERCRAFT_CONFIG_DIR=Join-Path $work 'settings'
$env:PYTHONDONTWRITEBYTECODE='1'
$preserved=Join-Path $env:APPDATA ('AfterCraft\installer-preservation-'+[Guid]::NewGuid().ToString('N')+'.txt')
$installed=$false
try {
    $install=Start-Process -FilePath $installerPath -WindowStyle Hidden -Wait -PassThru -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-',('/DIR="'+$target+'"'),'/TASKS="desktopicon,openwith"',('/LOG="'+(Join-Path $work 'install.log')+'"'))
    if($install.ExitCode -ne 0) {throw "Installer exited $($install.ExitCode); see $work"}
    $installed=$true
    foreach($name in @('AfterCraft.exe','aftercraft-cli.exe','effectcraft-aex-probe.exe','effectcraft-helper-ofx.exe','runtime\python\python.exe','aep-import\aep2effectcraft.py','licenses\rust-dependencies\inventory.json')) {
        if(!(Test-Path -LiteralPath (Join-Path $target $name))) {throw "Missing installed file: $name"}
    }
    $exe=Join-Path $target 'AfterCraft.exe'
    if((Get-Item -LiteralPath $exe).VersionInfo.ProductName -ne 'AfterCraft') {throw 'Wrong executable product identity'}
    $version=& $exe --version
    if($LASTEXITCODE -ne 0 -or $version -notmatch '^AfterCraft 0\.16\.0') {throw 'Desktop executable startup failed'}
    & (Join-Path $target 'aftercraft-cli.exe') --version
    if($LASTEXITCODE -ne 0) {throw 'CLI startup failed'}
    & (Join-Path $target 'runtime\python\python.exe') -c 'import py_aep; print("Installed parser ready")'
    if($LASTEXITCODE -ne 0) {throw 'Installed Python/parser failed'}
    $shell=New-Object -ComObject WScript.Shell
    foreach($link in @($startLink,$desktopLink)) {
        if(!(Test-Path -LiteralPath $link) -or $shell.CreateShortcut($link).TargetPath -ne $exe) {throw "Wrong shortcut: $link"}
    }
    if((Get-Item -LiteralPath $appPathKey).GetValue('') -ne $exe) {throw 'Wrong App Paths registration'}
    New-Item -ItemType Directory -Force -Path (Split-Path $preserved -Parent) | Out-Null
    Set-Content -LiteralPath $preserved -Value 'Original installer test data; uninstall must preserve it.'
    $remove=Start-Process -FilePath (Join-Path $target 'unins000.exe') -WindowStyle Hidden -Wait -PassThru -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/LOG="'+(Join-Path $work 'uninstall.log')+'"'))
    if($remove.ExitCode -ne 0) {throw "Uninstaller exited $($remove.ExitCode)"}
    $installed=$false
    if(Test-Path -LiteralPath $exe) {throw 'Uninstall left the main executable'}
    foreach($entry in @($uninstallKey,$appPathKey,$startLink,$desktopLink)) {
        if(Test-Path -LiteralPath $entry) {throw "Uninstall left registration or shortcut: $entry"}
    }
    if(!(Test-Path -LiteralPath $preserved)) {throw 'Uninstall removed user data'}
    foreach($file in $originals) {
        if((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ne $before[$file]) {throw "Normal EffectCraft changed: $file"}
    }
    [PSCustomObject]@{installedFiles=$true;desktopAndCliStart=$true;bundledParser=$true;shortcuts=$true;uninstall=$true;userDataPreserved=$true;normalEffectCraftUnchanged=$true;logs=$work} | ConvertTo-Json
} finally {
    $env:AFTERCRAFT_CONFIG_DIR=$oldConfig
    $env:PYTHONDONTWRITEBYTECODE=$oldBytecode
    if(Test-Path -LiteralPath $preserved) {Remove-Item -LiteralPath $preserved}
    if($installed) {Write-Warning "Smoke test stopped with its AfterCraft installation still at $target; inspect the logs before uninstalling it."}
}
