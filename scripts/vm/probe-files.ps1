# Phase 0 probe (risk d): starts hold-locks.ps1 in the desktop session, waits
# until the files are held, then runs the Rust probe on them and on files
# Windows itself keeps open.
$d = "$env:USERPROFILE\XkyHold-wd probe"
$hold = "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File C:\xkyvm\hold-locks.ps1 -Seconds 120"
powershell -NoProfile -ExecutionPolicy Bypass -File C:\xkyvm\in-desktop.ps1 -Exe powershell.exe -Arguments $hold -Wait 3 | Out-Null
for ($i = 0; ($i -lt 30) -and -not (Test-Path "$d\holding.flag"); $i++) { Start-Sleep 1 }
"holding: $(Test-Path "$d\holding.flag")"
$files = @('plain', 'locked-share-none', 'locked-share-read', 'locked-range', 'attr-offline', 'attr-hidden-system') |
    ForEach-Object { "$d\$_.txt" }
$files += "$env:USERPROFILE\NTUSER.DAT", "$env:LOCALAPPDATA\Microsoft\Windows\UsrClass.dat"
& C:\xkyvm\wd\target\debug\xkyhold-wd-probe-files.exe @files
