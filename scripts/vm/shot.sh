#!/bin/bash
# shot.sh <name> : screenshot of the VM desktop into the scratchpad.
S=${XKYVM_SCRATCH:-$(mktemp -d)}
ssh xkyvm 'powershell -NoProfile -ExecutionPolicy Bypass -File C:\xkyvm\in-desktop.ps1 -Exe powershell.exe -Arguments "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File C:\xkyvm\screenshot.ps1 -Out C:\xkyvm\shot.png" -Wait 30' >/dev/null && scp -q xkyvm:C:/xkyvm/shot.png "$S/$1.png" && echo "$S/$1.png"
