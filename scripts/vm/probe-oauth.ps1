# Phase 0 probe (risk c): runs the OAuth probe in the desktop session with
# the client id and secret read from C:\xkyvm\.env.probe (copied from the
# host's .env.local, deleted at the end). Output goes to probe-oauth.txt.
param([switch]$Forget)
$envFile = 'C:\xkyvm\.env.probe'
$out = 'C:\xkyvm\probe-oauth.txt'
if (Test-Path $envFile) {
    foreach ($line in Get-Content $envFile) {
        if ($line -match '^(XKYHOLD_WD_CLIENT_(ID|SECRET))=(.+)$') { Set-Item "env:$($Matches[1])" $Matches[3] }
    }
}
$exe = 'C:\xkyvm\wd\target\debug\xkyhold-wd-probe-oauth.exe'
try {
    if ($Forget) { & $exe --forget *> $out } else { & $exe *> $out }
    "exit code: $LASTEXITCODE" | Add-Content $out
} finally {
    Remove-Item $envFile -ErrorAction SilentlyContinue
}
