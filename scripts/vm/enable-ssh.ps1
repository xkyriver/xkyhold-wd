# Turns on the OpenSSH server in the throwaway test VM and authorizes the
# public key vm_key.pub that sits next to this script. Run once in an
# administrator PowerShell:
#
#   powershell -ExecutionPolicy Bypass -File \\host.lan\Data\enable-ssh.ps1
#
# The key belongs to the account that builds XkyHold-wd; only for the VM.

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$logs = Join-Path $here 'logs'
New-Item -ItemType Directory -Force -Path $logs | Out-Null
Start-Transcript -Path (Join-Path $logs ("enable-ssh-{0}.log" -f (Get-Date -Format 'yyyyMMdd-HHmmss'))) | Out-Null
try {
    # The Windows Update route (Add-WindowsCapability) stalled for over
    # 30 minutes in the test VM, so the server comes from winget instead
    # (Microsoft's OpenSSH MSI, published on GitHub). winget asks for its
    # agreements in this window.
    if (-not (Get-Service sshd -ErrorAction SilentlyContinue)) {
        winget install --id Microsoft.OpenSSH.Preview --exact --source winget
        if (-not (Get-Service sshd -ErrorAction SilentlyContinue)) { throw 'sshd was not installed' }
    }
    Set-Service -Name sshd -StartupType Automatic
    Start-Service sshd

    # Administrators read their keys from this file, not from ~\.ssh.
    $auth = Join-Path $env:ProgramData 'ssh\administrators_authorized_keys'
    Copy-Item -Force (Join-Path $here 'vm_key.pub') $auth
    icacls.exe $auth /inheritance:r /grant '*S-1-5-32-544:F' /grant '*S-1-5-18:F' | Out-Null

    # PowerShell as the login shell.
    New-ItemProperty -Path 'HKLM:\SOFTWARE\OpenSSH' -Name DefaultShell `
        -Value "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -PropertyType String -Force | Out-Null

    if (-not (Get-NetFirewallRule -Name 'OpenSSH-Server-In-TCP' -ErrorAction SilentlyContinue)) {
        New-NetFirewallRule -Name 'OpenSSH-Server-In-TCP' -DisplayName 'OpenSSH Server (sshd)' `
            -Enabled True -Direction Inbound -Protocol TCP -Action Allow -LocalPort 22 | Out-Null
    }
    Restart-Service sshd
    Write-Host "SSH is on. User: $env:USERNAME" -ForegroundColor Green
} catch {
    Write-Host "Failed: $_" -ForegroundColor Red
} finally {
    Stop-Transcript | Out-Null
}
Read-Host 'Press Enter to close'
