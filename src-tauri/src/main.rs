// No console window behind the app in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

const USAGE: &str = "Usage: xkyhold-wd [--scheduled | --help | --version]

  (no option)   open the window
  --scheduled   what the hourly scheduled task runs: a backup or a reminder, if due
  --help        show this help
  --version     show the version

For backups from a terminal, use xkyhold-wd-cli.";

fn main() -> std::process::ExitCode {
    use std::process::ExitCode;
    // Anything but the window answers here and leaves: no web view.
    match std::env::args_os().nth(1) {
        None => {}
        // The hourly Task Scheduler task (PLAN.md 4.8).
        Some(a) if a == "--scheduled" => {
            return match xkyhold_wd_app::schedule::run_scheduled() {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    xkyhold_wd_app::log(&format!("scheduled run: {e}"));
                    ExitCode::FAILURE
                }
            };
        }
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
