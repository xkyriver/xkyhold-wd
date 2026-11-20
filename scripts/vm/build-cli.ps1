# Builds xkyhold-wd-cli in the test VM with the Google client built in,
# read from C:\xkyvm\.env.probe (a copy of the host's .env.local), which is
# deleted at the end whatever happens.
$envFile = 'C:\xkyvm\.env.probe'
try {
    foreach ($line in Get-Content $envFile) {
        if ($line -match '^(XKYHOLD_WD_CLIENT_(ID|SECRET))=(.+)$') { Set-Item "env:$($Matches[1])" $Matches[3] }
    }
    $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
    Set-Location C:\xkyvm\wd
    cargo build -q -p xkyhold-wd-cli 2>&1 | Select-Object -Last 20
    "built: $(Test-Path C:\xkyvm\wd\target\debug\xkyhold-wd-cli.exe)"
} finally {
    Remove-Item $envFile -ErrorAction SilentlyContinue
}
