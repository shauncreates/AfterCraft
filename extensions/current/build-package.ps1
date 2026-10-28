param([string]$Output='')
$ErrorActionPreference='Stop'
$root=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
if(!$Output) {$Output=Join-Path $root 'artifacts/current-extension'}
$outputPath=[IO.Path]::GetFullPath($Output)
$folder=Join-Path $outputPath 'Current-1.0.0-preview.2'
New-Item -ItemType Directory -Force -Path $folder | Out-Null
foreach($name in @('README.md','NOTICE.md','Install-Current.ps1','test-current.cjs')) {Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination $folder}
foreach($name in @('LICENSE-MIT','LICENSE-APACHE')) {Copy-Item -LiteralPath (Join-Path $root $name) -Destination $folder}
Copy-Item -LiteralPath (Join-Path $root 'extensions/scriptui-panels/Current.jsx') -Destination (Join-Path $folder 'Current.jsx')
Copy-Item -LiteralPath (Join-Path $root 'extensions/scriptui-panels/Current.jsx.attribution') -Destination $folder
$archive=Join-Path $outputPath 'Current-1.0.0-preview.2.zip'
Compress-Archive -LiteralPath $folder -DestinationPath $archive -Force
$hash=(Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText((Join-Path $outputPath 'Current-1.0.0-preview.2-SHA256SUMS.txt'),$hash+'  '+[IO.Path]::GetFileName($archive)+"`n",[Text.UTF8Encoding]::new($false))
[PSCustomObject]@{Folder=$folder;Archive=$archive;SHA256=$hash}
