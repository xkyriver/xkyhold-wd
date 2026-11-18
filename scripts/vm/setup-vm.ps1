# Prepares a Windows 11 test VM to build XkyHold-wd (Rust, Node, Git,
# Visual Studio Build Tools). Run it from the shared folder in a normal
# PowerShell window:
#
#   powershell -ExecutionPolicy Bypass -File \\host.lan\Data\setup-vm.ps1
#
# The person at the VM accepts every license: winget asks for its own
# agreements and the Build Tools step asks before it starts. With
# -AcceptLicenses (only when the owner has accepted them beforehand) it runs
# without questions, for the job runner. A transcript
# is written next to the script, in logs\, so the result can be read from
# the host.

param([switch]$AcceptLicenses)

$ErrorActionPreference = 'Continue'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$logs = Join-Path $here 'logs'
New-Item -ItemType Directory -Force -Path $logs | Out-Null
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
Start-Transcript -Path (Join-Path $logs "setup-$stamp.log") | Out-Null

$RustVersion = '1.100.0'
$NodeVersion = '26.8.1'

function Step($text) { Write-Host ""; Write-Host "=== $text" -ForegroundColor Cyan }

function Refresh-Path {
    $machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $user = [Environment]::GetEnvironmentVariable('Path', 'User')
    $env:Path = "$machine;$user"
}

function Winget-Install($id, $extra) {
    $listed = winget list --id $id --exact --accept-source-agreements 2>$null | Select-String -SimpleMatch $id
    if ($listed) { Write-Host "$id is already installed."; return }
    $wargs = @('install', '--id', $id, '--exact', '--source', 'winget') + $extra
    if ($AcceptLicenses) { $wargs += @('--accept-package-agreements', '--accept-source-agreements', '--disable-interactivity') }
    & winget @wargs
    Write-Host "winget exit code for ${id}: $LASTEXITCODE"
}

Step 'Windows'
Get-ComputerInfo -Property OsName, OsVersion, OsBuildNumber, OsArchitecture | Format-List

Step 'winget'
# Over SSH the WindowsApps alias is not on PATH; call the package directly.
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    $found = Get-ChildItem "$env:ProgramFiles\WindowsApps\Microsoft.DesktopAppInstaller_*_x64__8wekyb3d8bbwe\winget.exe" -ErrorAction SilentlyContinue |
        Sort-Object FullName | Select-Object -Last 1
    if ($found) { Set-Alias -Name winget -Value $found.FullName -Scope Script }
}
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Host 'winget is missing. Update "App Installer" from the Microsoft Store and run this script again.' -ForegroundColor Red
    Stop-Transcript | Out-Null
    exit 1
}
winget --version

Step 'Git'
Winget-Install 'Git.Git' @()

Step "Node.js $NodeVersion"
Winget-Install 'OpenJS.NodeJS' @('--version', $NodeVersion)
if ($LASTEXITCODE -ne 0) {
    Write-Host "Node $NodeVersion is not in winget; installing the current release instead."
    Winget-Install 'OpenJS.NodeJS' @()
}

Step 'Rustup'
Winget-Install 'Rustlang.Rustup' @()

Step 'Visual Studio 2022 Build Tools (C++ workload and Windows SDK)'
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$haveVc = $false
if (Test-Path $vswhere) {
    $haveVc = [bool](& $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
}
if ($haveVc) {
    Write-Host 'The C++ build tools are already installed.'
} else {
    Write-Host 'This installs Microsoft Visual Studio 2022 Build Tools, which comes with'
    Write-Host 'Microsoft license terms: https://visualstudio.microsoft.com/license-terms/vs2022-ga-diagnosticbuildtools/'
    Write-Host 'Windows will also ask for permission (UAC).'
    $answer = if ($AcceptLicenses) { 'YES' } else { Read-Host 'Type YES to accept the license and install' }
    if ($answer -ceq 'YES') {
        Winget-Install 'Microsoft.VisualStudio.2022.BuildTools' @('--override', '--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended')
    } else {
        Write-Host 'Build Tools skipped.' -ForegroundColor Yellow
    }
}

Refresh-Path

Step "Rust $RustVersion"
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    rustup toolchain install $RustVersion --profile minimal --component rustfmt,clippy
    rustup default $RustVersion
} else {
    Write-Host 'rustup is not on PATH yet; open a new PowerShell and run this script again.' -ForegroundColor Yellow
}

Step 'Versions'
foreach ($cmd in @('git --version', 'node --version', 'npm --version', 'rustup --version', 'rustc --version', 'cargo --version')) {
    try { Write-Host "$cmd -> $(Invoke-Expression $cmd 2>&1 | Select-Object -First 1)" } catch { Write-Host "$cmd -> missing" }
}
if (Test-Path $vswhere) {
    Write-Host "VC tools: $(& $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)"
}
$wv = Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue
Write-Host "WebView2: $(if ($wv) { $wv.pv } else { 'not found' })"

Step 'Done'
Stop-Transcript | Out-Null
