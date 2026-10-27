param([Parameter(Mandatory=$true)][string]$Destination,[string]$Python='python',[string]$Archive='')
$ErrorActionPreference='Stop'
$root=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$tools=Join-Path $root '.build-tools'
New-Item -ItemType Directory -Force -Path $tools | Out-Null
if (!$Archive) {
    $Archive=Join-Path $tools 'python-3.13.17-embed-amd64.zip'
    if (!(Test-Path -LiteralPath $Archive)) {Invoke-WebRequest 'https://www.python.org/ftp/python/3.13.17/python-3.13.17-embed-amd64.zip' -OutFile $Archive}
}
if ((Get-FileHash -LiteralPath $Archive).Hash -ne '9E35319BFFEDAB8B36AD44D8F377CB4F1733BCEC257A0B6D04D94F01486DDB38') {throw 'Embedded Python checksum mismatch'}
New-Item -ItemType Directory -Force -Path $Destination | Out-Null
Expand-Archive -LiteralPath $Archive -DestinationPath $Destination -Force
$site=Join-Path $Destination 'Lib\site-packages'
& $Python -m pip install --disable-pip-version-check --no-compile --only-binary=:all: --platform win_amd64 --python-version 3.13 --implementation cp --abi cp313 --target $site -r (Join-Path $PSScriptRoot 'requirements.txt')
if ($LASTEXITCODE -ne 0) {throw 'Cannot prepare the private AE parser dependencies'}
@('python313.zip','.','Lib/site-packages','import site') | Set-Content (Join-Path $Destination 'python313._pth') -Encoding ascii
& (Join-Path $Destination 'python.exe') -B -c 'import py_aep; print("Bundled AE parser ready")'
if ($LASTEXITCODE -ne 0) {throw 'Bundled Python runtime did not start'}
