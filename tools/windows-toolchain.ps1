# Shared native linker setup for Windows builds. Does not change app/user settings.
function Initialize-AfterCraftToolchain {
    param([string]$Cargo='cargo')
    $ErrorActionPreference='Stop'
    $sysroot=(& rustc --print sysroot).Trim()
    if ($LASTEXITCODE -ne 0) {throw 'Rust toolchain unavailable'}
    $hostTriple=(& rustc -vV | Select-String '^host: ').ToString().Substring(6).Trim()
    if ($hostTriple -eq 'x86_64-pc-windows-gnu') {
        $llvm=Join-Path $sysroot 'lib\rustlib\x86_64-pc-windows-gnu\bin'
        $lld=Join-Path $llvm 'ld.lld.exe'
        if (!(Test-Path -LiteralPath $lld)) {Copy-Item -LiteralPath (Join-Path $llvm 'rust-lld.exe') -Destination $lld}
        $dlltool=Join-Path $llvm 'llvm-dlltool.exe'
        if (!(Test-Path -LiteralPath $dlltool)) {Copy-Item -LiteralPath (Join-Path $llvm 'llvm-ar.exe') -Destination $dlltool}
        $env:PATH=(Join-Path $llvm 'self-contained')+';'+$llvm+';'+$env:PATH
        Remove-Item Env:CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
        $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-C linker-flavor=ld -C linker=$lld -C dlltool=$dlltool -C link-arg=-s"
    }
    return $Cargo
}
