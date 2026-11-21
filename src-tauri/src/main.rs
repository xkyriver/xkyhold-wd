// No console window behind the app in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

const USAGE: &str = "Usage: xkyhold-wd [--help | --version]

  (no option)   open the window
  --help        show this help
  --version     show the version

For backups from a terminal, use xkyhold-wd-cli.";

fn main() -> std::process::ExitCode {
    use std::process::ExitCode;
    match std::env::args_os().nth(1) {
        None => {}
        Some(a) if a == "--help" || a == "-h" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some(a) if a == "--version" || a == "-V" => {
            println!("xkyhold-wd {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some(a) => {
            eprintln!(
                "xkyhold-wd: unknown option {}\n\n{USAGE}",
                a.to_string_lossy()
            );
            return ExitCode::from(2);
        }
    }
    xkyhold_wd_lib::run();
    ExitCode::SUCCESS
}
