# Phase 0 probe (risk d): reads one OneDrive test file from the desktop
# session, as the real app would, and records the attributes before and after.
param([string]$Name = 'cloud-b.bin', [string]$Out = 'C:\xkyvm\probe-cloud.txt')
$d = Join-Path $env:OneDrive 'XkyHold-wd probe'
$p = 'C:\xkyvm\wd\target\debug\xkyhold-wd-probe-files.exe'
& {
    "before: 0x{0:x}" -f [int](Get-Item "$d\$Name").Attributes
    $t = Measure-Command { & $p "$d\$Name" | Out-String | Write-Output -OutVariable script:o }
    $script:o
    "read took $([int]$t.TotalMilliseconds) ms"
    "after: 0x{0:x}" -f [int](Get-Item "$d\$Name").Attributes
    "other file (not read): 0x{0:x}" -f [int](Get-Item "$d\cloud-a.bin").Attributes
} *> $Out
