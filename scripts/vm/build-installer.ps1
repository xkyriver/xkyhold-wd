# Builds the XkyHold-wd release and its NSIS installer in the test VM with the
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
    npx.cmd tauri build 2>&1 | Select-Object -Last 25
    Get-ChildItem C:\xkyvm\wd\target\release\bundle\nsis -Filter *.exe | ForEach-Object { "built: $($_.Name) $($_.Length)" }
} finally {
    Remove-Item $envFile -ErrorAction SilentlyContinue
}
