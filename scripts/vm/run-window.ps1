# Starts the debug window with the probe folder, from the desktop session.
$env:XKYHOLD_WD_PROBE = 'C:\xkyvm\probe'
New-Item -ItemType Directory -Force C:\xkyvm\probe | Out-Null
Start-Process -FilePath C:\xkyvm\wd\target\debug\xkyhold-wd.exe -WorkingDirectory C:\xkyvm -RedirectStandardError C:\xkyvm\window-stderr.txt
