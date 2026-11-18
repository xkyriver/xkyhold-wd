# Phase 0 probe (risk d): holds test files open the way other programs do,
# for -Seconds, so a backup read can be tried against them. Runs in the
# desktop session through in-desktop.ps1. Only touches the given folder.
param([string]$Dir = "$env:USERPROFILE\XkyHold-wd probe", [int]$Seconds = 180)
$none = [IO.File]::Open("$Dir\locked-share-none.txt", 'Open', 'ReadWrite', 'None')
$readOnly = [IO.File]::Open("$Dir\locked-share-read.txt", 'Open', 'ReadWrite', 'Read')
$range = [IO.File]::Open("$Dir\locked-range.txt", 'Open', 'ReadWrite', 'ReadWrite')
$range.Lock(0, $range.Length)
Set-Content "$Dir\holding.flag" 'yes'
Start-Sleep -Seconds $Seconds
$range.Unlock(0, $range.Length)
$none.Close(); $readOnly.Close(); $range.Close()
Remove-Item "$Dir\holding.flag"
