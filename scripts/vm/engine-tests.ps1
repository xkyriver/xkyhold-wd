# Runs every test binary of the engine in the desktop session (a normal
# user, not elevated: the engine refuses to back up as administrator).
# Build them first over SSH with `cargo test --workspace --no-run`.
# Output in C:\xkyvm\engine-tests.txt.
$out = 'C:\xkyvm\engine-tests.txt'
$target = 'C:\xkyvm\engine\target\debug'
& {
    $admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    "elevated: $admin"
    Get-ChildItem $target -Recurse -Filter '*.exe' |
        Where-Object { $_.Name -match '^[a-z0-9_]+-[0-9a-f]{16}\.exe$' -and $_.Name -notlike 'build_script*' } |
        Sort-Object Name -Unique |
        ForEach-Object { "== $($_.Name)"; & $_.FullName --test-threads 1 2>&1 }
    "done"
} *> $out
