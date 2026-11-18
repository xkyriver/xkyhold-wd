# Runs a command in the logged-on desktop session of the test VM, from an
# SSH session (which has no desktop). Uses a one-off scheduled task with an
# interactive logon, so no password is involved.
#
#   powershell -ExecutionPolicy Bypass -File in-desktop.ps1 -Exe <path> [-Arguments <args>]
param(
    [Parameter(Mandatory)] [string]$Exe,
    [string]$Arguments = '',
    [int]$Wait = 60
)
$name = 'xkyhold-wd-in-desktop-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
$action = if ($Arguments) { New-ScheduledTaskAction -Execute $Exe -Argument $Arguments } else { New-ScheduledTaskAction -Execute $Exe }
$principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive
Register-ScheduledTask -TaskName $name -Action $action -Principal $principal -Force | Out-Null
Start-ScheduledTask -TaskName $name
# Wait for the task to end (up to -Wait seconds), so that unregistering it
# does not stop it; GUI programs that keep running are left alone.
$deadline = (Get-Date).AddSeconds($Wait)
do {
    Start-Sleep -Seconds 1
    $state = (Get-ScheduledTask -TaskName $name).State
} while ($state -eq 'Running' -and (Get-Date) -lt $deadline)
$info = Get-ScheduledTaskInfo -TaskName $name
Write-Host "State: $state, last result: $($info.LastTaskResult)"
if ($state -ne 'Running') { Unregister-ScheduledTask -TaskName $name -Confirm:$false }
