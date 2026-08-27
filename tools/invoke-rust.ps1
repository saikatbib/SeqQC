param(
    [Parameter(Position = 0, ValueFromRemainingArguments = $true)]
    [string[]] $CargoArguments = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$toolchainBin = Join-Path $repositoryRoot '.toolchains\rustup\toolchains\1.98.0-x86_64-pc-windows-msvc\bin'
$cargoExecutable = Join-Path $toolchainBin 'cargo.exe'
$rustcExecutable = Join-Path $toolchainBin 'rustc.exe'
$vswhereExecutable = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$strawberryPerlBin = 'C:\Strawberry\perl\bin'

if (-not (Test-Path -LiteralPath $cargoExecutable)) {
    throw 'Pinned Rust 1.98.0 toolchain was not found under .toolchains.'
}
if (-not (Test-Path -LiteralPath $vswhereExecutable)) {
    throw 'Visual Studio Build Tools locator was not found.'
}

$visualStudioPath = & $vswhereExecutable -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $visualStudioPath) {
    throw 'Visual Studio 2022 C++ Build Tools workload was not found.'
}

$developerShellModule = Join-Path $visualStudioPath 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll'
Import-Module $developerShellModule
Enter-VsDevShell -VsInstallPath $visualStudioPath -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64'

$env:RUSTUP_HOME = Join-Path $repositoryRoot '.toolchains\rustup'
$env:CARGO_HOME = Join-Path $repositoryRoot '.toolchains\cargo'
$env:RUSTC = $rustcExecutable
$env:PATH = "$toolchainBin;$env:PATH"
if ((-not (Get-Command perl -ErrorAction SilentlyContinue)) -and
    (Test-Path -LiteralPath (Join-Path $strawberryPerlBin 'perl.exe'))) {
    $env:PATH = "$strawberryPerlBin;$env:PATH"
}

Push-Location $repositoryRoot
try {
    & $cargoExecutable @CargoArguments
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
