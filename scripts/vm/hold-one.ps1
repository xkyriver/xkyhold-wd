# Holds one file open with no sharing for -Seconds (as a program that keeps
# its file locked), from the desktop session.
param([string]$File = "$env:USERPROFILE\Documents\XkyHold-wd ui test\in-use.txt", [int]$Seconds = 120)
$h = [IO.File]::Open($File, 'Open', 'ReadWrite', 'None')
Start-Sleep -Seconds $Seconds
$h.Close()
