#Requires -Version 7.0
<#
仅构建与测试本地原型；不安装工具链，不启动或注册输入法，不触碰现有配置。
#>
[CmdletBinding()]
param(
    [string] $ToolchainRoot,
    [string] $BuildRoot
)

$ErrorActionPreference = 'Stop'
$taskRepo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$taskOutput = Join-Path $taskRepo 'target\english-to-chinese'
New-Item -ItemType Directory -Path $taskOutput -Force | Out-Null
$taskStatusPath = Join-Path $taskOutput 'build-status.json'
$taskOldRustupHome = $env:RUSTUP_HOME
$taskOldPath = $env:PATH
$taskOldTarget = $env:CARGO_TARGET_DIR
$taskOldDevDebug = $env:CARGO_PROFILE_DEV_DEBUG
$taskOldTestDebug = $env:CARGO_PROFILE_TEST_DEBUG
$taskOldIncremental = $env:CARGO_INCREMENTAL
if ($ToolchainRoot) {
    $taskTools = (Resolve-Path -LiteralPath $ToolchainRoot).Path
    $env:RUSTUP_HOME = Join-Path $taskTools 'rustup'
    $env:PATH = (Join-Path $taskTools 'cargo\bin') + ';' + $env:PATH
}
$taskCargoTarget = if ($BuildRoot) { Join-Path $BuildRoot 'target' } else { Join-Path $taskRepo 'target' }
$taskCargoCache = if ($BuildRoot) { Join-Path $BuildRoot 'cargo-home' } else { Join-Path $taskRepo 'target\cargo-home' }
$taskMissing = [System.Collections.Generic.List[string]]::new()
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { $taskMissing.Add('Cargo') }
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) { $taskMissing.Add('Rust 1.96.0 compiler') }
$taskRustup = Get-Command rustup -ErrorAction SilentlyContinue
if ($taskRustup) {
    $taskInstalled = & $taskRustup.Source toolchain list
    if (-not ($taskInstalled -match '^1\.96\.0-')) { $taskMissing.Add('installed Rust 1.96.0 toolchain (no automatic installation)') }
}
$taskVswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$taskVs = if (Test-Path -LiteralPath $taskVswhere) {
    & $taskVswhere -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
}
if (-not $taskVs) { $taskMissing.Add('Visual Studio MSVC C++ x86/x64 build tools') }
$taskSdkRoots = @(
    'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows Kits\Installed Roots'
) | ForEach-Object { (Get-ItemProperty -LiteralPath $_ -ErrorAction SilentlyContinue).KitsRoot10 }
$taskSdk = $taskSdkRoots | Where-Object {
    $_ -and (Test-Path (Join-Path $_ 'Lib\*\um\x64\kernel32.lib'))
}
if (-not $taskSdk) { $taskMissing.Add('Windows 10/11 SDK x64 libraries') }

if ($taskMissing.Count -gt 0) {
    [ordered]@{ status = 'blocked'; reason = 'missing_toolchain'; missing = $taskMissing.ToArray()
        rust_tests_executed = $false; binaries_built = $false; input_method_installed = $false } |
        ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $taskStatusPath -Encoding utf8
    Write-Output ('Build blocked: ' + ($taskMissing -join '; '))
    Write-Output ('Evidence: ' + $taskStatusPath)
    exit 2
}

function Invoke-CheckedCargo([string[]] $CargoArguments) {
    Add-Content -LiteralPath (Join-Path $taskOutput 'cargo.log') -Value ('cargo ' + ($CargoArguments -join ' '))
    & cargo @CargoArguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOutput 'cargo.log') -Append
    if ($LASTEXITCODE -ne 0) { throw ('cargo failed: ' + ($CargoArguments -join ' ')) }
}

$taskOldCargoHome = $env:CARGO_HOME
$taskOldUiAccess = $env:QINGJIAN_UIACCESS
Push-Location -LiteralPath $taskRepo
try {
    # 构建缓存可放到空间较充足的盘；只改当前进程，输出清单仍留在项目里。
    $env:CARGO_HOME = $taskCargoCache
    $env:CARGO_TARGET_DIR = $taskCargoTarget
    if ($BuildRoot) {
        $env:CARGO_PROFILE_DEV_DEBUG = '0'
        $env:CARGO_PROFILE_TEST_DEBUG = '0'
        $env:CARGO_INCREMENTAL = '0'
    }
    $env:QINGJIAN_UIACCESS = '0'
    Start-Transcript -LiteralPath (Join-Path $taskOutput 'build.log') -Force | Out-Null
    Invoke-CheckedCargo -CargoArguments @('fmt', '--all', '--', '--check')
    Invoke-CheckedCargo -CargoArguments @('test', '--locked', '-p', 'qingjian-core', '-p', 'qingjian-translate', '-p', 'qingjian-platform')
    Invoke-CheckedCargo -CargoArguments @('test', '--locked', '-p', 'qingjian-windows-server', '-p', 'qingjian-windows-tsf')
    Invoke-CheckedCargo -CargoArguments @('test', '--locked', '-p', 'qingjian-settings', 'support::tests::')
    Invoke-CheckedCargo -CargoArguments @('clippy', '--locked', '-p', 'qingjian-core', '-p', 'qingjian-translate', '-p', 'qingjian-platform', '-p', 'qingjian-cli', '-p', 'qingjian-windows-server', '-p', 'qingjian-windows-tsf', '-p', 'qingjian-settings', '--all-targets', '--', '-D', 'warnings')
    Invoke-CheckedCargo -CargoArguments @('build', '--locked', '--release', '-p', 'qingjian-cli', '-p', 'qingjian-windows-server', '-p', 'qingjian-windows-tsf')
    # CLI 只查询固定公开样例；未指定 --predict 或个人配置，不启动 Server。
    & (Join-Path $taskCargoTarget 'release\qingjian-cli.exe') --english-to-chinese studio thwart set apple develop bank right privacy 'set a record' 2>&1 | Tee-Object -FilePath (Join-Path $taskOutput 'cli-smoke.log')
    if ($LASTEXITCODE -ne 0) { throw 'Fixed CLI smoke query failed' }
    $taskBinOutput = Join-Path $taskOutput 'bin'
    New-Item -ItemType Directory -Path $taskBinOutput -Force | Out-Null
    $taskArtifacts = foreach ($taskName in @('qingjian-server.exe', 'qingjian_tsf.dll', 'qingjian-cli.exe')) {
        $taskBuilt = Join-Path $taskCargoTarget ('release\' + $taskName)
        $taskSaved = Join-Path $taskBinOutput $taskName
        Copy-Item -LiteralPath $taskBuilt -Destination $taskSaved -Force
        [ordered]@{ path = $taskSaved; sha256 = (Get-FileHash -LiteralPath $taskSaved -Algorithm SHA256).Hash }
    }
    [ordered]@{ status = 'built'; rust_tests_executed = $true; binaries_built = $true
        input_method_installed = $false; manual_tsf_acceptance = 'pending'; artifacts = @($taskArtifacts) } |
        ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $taskStatusPath -Encoding utf8
}
catch {
    [ordered]@{ status = 'failed'; error = $_.Exception.Message; input_method_installed = $false } |
        ConvertTo-Json | Set-Content -LiteralPath $taskStatusPath -Encoding utf8
    throw
}
finally {
    Stop-Transcript -ErrorAction SilentlyContinue | Out-Null
    $env:CARGO_HOME = $taskOldCargoHome
    $env:QINGJIAN_UIACCESS = $taskOldUiAccess
    $env:RUSTUP_HOME = $taskOldRustupHome
    $env:PATH = $taskOldPath
    $env:CARGO_TARGET_DIR = $taskOldTarget
    $env:CARGO_PROFILE_DEV_DEBUG = $taskOldDevDebug
    $env:CARGO_PROFILE_TEST_DEBUG = $taskOldTestDebug
    $env:CARGO_INCREMENTAL = $taskOldIncremental
    Pop-Location
}
