# Moves the OneDrive test folder of the risk (d) probe to the Recycle Bin
# (OneDrive keeps it in its own recycle bin too). Runs in the desktop session.
Add-Type -AssemblyName Microsoft.VisualBasic
$d = Join-Path $env:OneDrive 'XkyHold-wd probe'
try {
    [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory($d, 'OnlyErrorDialogs', 'SendToRecycleBin')
    "trashed; exists: $(Test-Path $d)"
} catch { "failed: $_" }
