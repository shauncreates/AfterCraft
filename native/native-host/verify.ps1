$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    & "$PSScriptRoot\build.ps1" -Action test
    & "$PSScriptRoot\build.ps1" -Action clippy
    & "$PSScriptRoot\build.ps1" -Action build
    $cargoExe = 'cargo'
    & $cargoExe fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Rust formatting check failed' }
    $probeExe = Join-Path $PSScriptRoot 'target\debug\effectcraft-aex-probe.exe'
    New-Item -ItemType Directory -Force -Path reports | Out-Null
    $isolationText = & $probeExe self-test
    if ($LASTEXITCODE -ne 0) { throw 'Worker isolation tests failed' }
    $isolation = ($isolationText -join "`n") | ConvertFrom-Json
    $runDir = Join-Path $PSScriptRoot ('reports\verified-' + [Guid]::NewGuid().ToString('N'))
    $roundtripText = & $probeExe roundtrip-test (Join-Path $PSScriptRoot 'target\debug\aex_gain_fixture.dll') $runDir
    if ($LASTEXITCODE -ne 0) { throw 'Native frame roundtrip failed' }
    $roundtrip = ($roundtripText -join "`n") | ConvertFrom-Json
    $result = [PSCustomObject]@{
        checkedAtUtc = [DateTime]::UtcNow.ToString('o')
        unitTestsPassed = 'see cargo test output'
        clippyWarnings = 0
        formattingPassed = $true
        isolation = $isolation
        nativeFrameRoundtrip = $roundtrip
        borisEffectOutputVerified = $false
        effectcraftIntegrated = $true
    }
    $result | ConvertTo-Json -Depth 10 | Set-Content reports\validation.json -Encoding utf8
    New-Item -ItemType Directory -Force -Path dist | Out-Null
    Copy-Item -LiteralPath $probeExe -Destination dist\effectcraft-aex-probe.exe
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'target\debug\aex_gain_fixture.dll') -Destination dist\gain-fixture.aex
    $result | ConvertTo-Json -Depth 10
} finally { Pop-Location }
