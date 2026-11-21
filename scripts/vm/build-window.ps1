# Builds the XkyHold-wd window (debug, no installer) in the test VM with the
# Google client built in, read from C:\xkyvm\.env.probe (a copy of the
# host's .env.local), which is deleted at the end whatever happens.
$envFile = 'C:\xkyvm\.env.probe'
try {
    foreach ($line in Get-Content $envFile) {
        if ($line -match '^(XKYHOLD_WD_CLIENT_(ID|SECRET))=(.+)$') { Set-Item "env:$($Matches[1])" $Matches[3] }
    }
    $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
    Set-Location C:\xkyvm\wd
    npm.cmd install --no-audit --no-fund 2>&1 | Select-Object -Last 3
    npx.cmd tauri build --debug --no-bundle 2>&1 | Select-Object -Last 25
    "built: $(Test-Path C:\xkyvm\wd\target\debug\xkyhold-wd.exe)"
} finally {
    Remove-Item $envFile -ErrorAction SilentlyContinue
}
