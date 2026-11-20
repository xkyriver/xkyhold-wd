//! `xkyhold-wd-cli`: back up and restore the Windows user folder from the
//! command line, for people who prefer a terminal and for scripts.
//!
//! The password comes from `XKYHOLD_WD_PASSWORD` or is asked on the
//! terminal; it is never stored.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use xkyhold_core::check::check;
use xkyhold_core::crypto::{Credential, DEFAULT_ARGON2};
use xkyhold_core::destination::Destination;
use xkyhold_core::repo::Repository;
use xkyhold_core::restore::{Conflict, RestoreOptions, restore};
use xkyhold_core::rules::PRESETS;
use xkyhold_core::snapshot::Snapshot;
use xkyhold_core::time::now_iso;
use xkyhold_core::tree::{Entry, EntryKind};
use xkyhold_core::{Error, Result};
use xkyhold_dest_drive::DriveFolder;
use xkyhold_wd_app::{LastBackup, Settings, Target, What, human};
use zeroize::Zeroizing;

#[derive(Parser)]
#[command(
    name = "xkyhold-wd-cli",
    version,
    about = "Encrypted backup of your Windows user folder"
)]
struct Cli {
    /// The backup: a folder on this computer, an external disk or a network share.
    /// Without --repo or --drive, the backup set up on this computer.
    #[arg(long, global = true, env = "XKYHOLD_WD_REPO", conflicts_with = "drive")]
    repo: Option<PathBuf>,
    /// The backup: the folder XkyHold/<NAME> in your Google Drive (see `login`).
    #[arg(long, global = true, env = "XKYHOLD_WD_DRIVE")]
    drive: Option<String>,
    /// Upload at most this many kilobytes per second to Google Drive.
    #[arg(long, global = true, value_name = "KB/s")]
    limit_upload: Option<u64>,
    /// Open with the recovery key instead of the password.
    #[arg(long, global = true)]
    recovery_key: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new backup and show its recovery key.
    Init,
    /// Back up your user folder, or only some items inside it.
    Backup {
        /// Back up only these items (paths inside your user folder).
        #[arg(long = "only")]
        only: Vec<PathBuf>,
        /// Leave out these items (paths inside your user folder).
        #[arg(long = "leave-out")]
        leave_out: Vec<PathBuf>,
        /// Switch on a suggested exclusion (see `presets`).
        #[arg(long = "preset")]
        presets: Vec<String>,
        /// Leave out what matches a pattern (`**/*.iso`).
        #[arg(long = "pattern")]
        patterns: Vec<String>,
    },
    /// List the backups made.
    Snapshots,
    /// List what a backup holds.
    Ls {
        /// Snapshot id (or the start of it), or `latest`.
        snapshot: String,
        /// A folder inside the backup.
        path: Option<String>,
    },
    /// Restore from a backup into a folder.
    Restore {
        snapshot: String,
        /// Items to restore (paths inside the backup); everything by default.
        paths: Vec<String>,
        /// Where to restore: any folder you can write to, inside or outside your user
        /// folder (an external disk, for example). It is created if missing.
        #[arg(long)]
        to: PathBuf,
        #[arg(long, value_enum, default_value_t = ConflictArg::KeepBoth)]
        conflict: ConflictArg,
    },
    /// Check that the backup is complete and intact.
    Check {
        /// Also download and verify every piece of data.
        #[arg(long)]
        full: bool,
    },
    /// Remove backups older than some months (always keeps the latest) and free their space.
    Forget {
        #[arg(long)]
        keep_months: u32,
    },
    /// Show the state of the backup.
    Status,
    /// List the suggested exclusions.
    Presets,
    /// Connect to your Google Drive (opens the browser).
    Login {
        /// Print the sign-in address instead of opening the browser.
        #[arg(long)]
        no_browser: bool,
    },
    /// Disconnect from Google Drive on this computer.
    Logout,
    /// List the backups in your Google Drive.
    Backups,
    /// Keep this backup's key in the Credential Manager, so backups need no password.
    Remember,
    /// Remove this backup's key from the Credential Manager.
    ForgetKey,
}

#[derive(Clone, Copy, ValueEnum)]
enum ConflictArg {
    Skip,
    Replace,
    KeepBoth,
}

type Repo = xkyhold_wd_app::Repo;

/// The backup named by `--repo` / `--drive`, else the one set up on this
/// computer (by the window or an earlier command).
fn target(cli: &Cli, settings: &Settings) -> Result<(Target, bool)> {
    if let Some(name) = &cli.drive {
        return Ok((Target::Drive { name: name.clone() }, false));
    }
    if let Some(path) = &cli.repo {
        return Ok((
            Target::Local {
                path: std::path::absolute(path)?,
            },
            false,
        ));
    }
    match &settings.backup {
        Some(t) => Ok((t.clone(), true)),
        None => Err(Error::Usage(
            "say where the backup is with --repo or --drive, or set it up in the window".into(),
        )),
    }
}

fn destination(cli: &Cli) -> Result<(Arc<dyn Destination>, Option<PathBuf>)> {
    let settings = Settings::load()?;
    let (target, _) = target(cli, &settings)?;
    xkyhold_wd_app::destination(&target, &settings, cli.limit_upload)
}

/// Opens with the password (or recovery key) when given, else with the
/// key kept in the keyring, else asks for the password.
fn open_dest(cli: &Cli, dest: Arc<dyn Destination>) -> Result<Repo> {
    let explicit = cli.recovery_key || std::env::var_os("XKYHOLD_WD_PASSWORD").is_some();
    if !explicit && let Some(repo) = xkyhold_wd_app::open_with_keyring(dest.clone())? {
        return Ok(repo);
    }
    xkyhold_wd_app::open_with(dest, &credential(cli)?)
}

fn ask(prompt: &str, env: &str) -> Result<Zeroizing<String>> {
    if let Ok(value) = std::env::var(env) {
        return Ok(Zeroizing::new(value));
    }
    Ok(Zeroizing::new(rpassword::prompt_password(prompt)?))
}

fn credential(cli: &Cli) -> Result<Credential> {
    Ok(if cli.recovery_key {
        Credential::RecoveryKey(ask("Recovery key: ", "XKYHOLD_WD_RECOVERY_KEY")?)
    } else {
        Credential::Password(ask("Password: ", "XKYHOLD_WD_PASSWORD")?)
    })
}

fn open(cli: &Cli) -> Result<Repo> {
    let (dest, _) = destination(cli)?;
    open_dest(cli, dest)
}

/// A line on stderr every few seconds while a long backup runs.
fn ticker() -> impl FnMut(&xkyhold_core::backup::Progress) {
    let mut last = std::time::Instant::now();
    move |p| {
        if last.elapsed().as_secs() >= 5 {
            last = std::time::Instant::now();
            eprintln!(
                "  {} files, {} read… {}",
                p.files,
                human(p.bytes),
                p.current
            );
        }
    }
}

fn split_path(path: &str) -> Vec<&[u8]> {
    path.split('/')
        .filter(|p| !p.is_empty())
        .map(str::as_bytes)
        .collect()
}

fn root_and_entries(repo: &Repo, snap: &Snapshot, path: Option<&str>) -> Result<Vec<Entry>> {
    let names = path.map(split_path).unwrap_or_default();
    xkyhold_wd_app::entries(repo, snap, &names).map_err(|e| match e {
        Error::NotFound(_) => Error::Usage(format!(
            "nothing at \"{}\" in this backup",
            path.unwrap_or_default()
        )),
        e => e,
    })
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Command::Login { no_browser } => {
            let account = xkyhold_wd_app::google_sign_in(&mut |url| {
                if *no_browser || !xkyhold_wd_app::open_in_browser(url) {
                    println!("Open this address in your browser to connect XkyHold-wd:\n\n{url}\n");
                } else {
                    println!(
                        "Your browser opened Google's sign-in page. Waiting for you to finish there…"
                    );
                }
            })?;
            let mut settings = Settings::load()?;
            settings.account = Some(account.clone());
            settings.save()?;
            println!("Connected to the Google Drive of {account}.");
        }
        Command::Logout => {
            let mut settings = Settings::load()?;
            if let Some(account) = settings.account.take() {
                xkyhold_wd_app::google_sign_out(&account)?;
                settings.save()?;
                println!("Disconnected from the Google Drive of {account} on this computer.");
            }
        }
        Command::Backups => {
            for name in DriveFolder::list_repositories(&xkyhold_wd_app::drive_client(
                &Settings::load()?,
                None,
            )?)? {
                println!("{name}");
            }
        }
        Command::Remember => {
            let (dest, _) = destination(cli)?;
            let repo = xkyhold_wd_app::open_with(dest, &credential(cli)?)?;
            xkyhold_wd_app::remember_key(&repo, true)?;
            println!("This computer now opens this backup without the password.");
        }
        Command::ForgetKey => {
            let (dest, _) = destination(cli)?;
            let config = Repository::read_config(&dest)?;
            xkyhold_wd_app::forget_key(&config.repo_id)?;
            println!("This computer will ask for the password again.");
        }
        Command::Presets => {
            for p in PRESETS {
                let beside = p
                    .beside
                    .map(|f| format!("  (only next to {f})"))
                    .unwrap_or_default();
                println!("{:16} {}{beside}", p.id, p.patterns.join("  "));
            }
        }
        Command::Init => {
            let (dest, _) = destination(cli)?;
            let password = ask("New password: ", "XKYHOLD_WD_PASSWORD")?;
            if std::env::var("XKYHOLD_WD_PASSWORD").is_err()
                && *ask("Repeat it: ", "-")? != *password
            {
                return Err(Error::Usage("the passwords are different".into()));
            }
            let (_, recovery) = Repository::create(
                dest,
                &password,
                DEFAULT_ARGON2,
                now_iso(),
                xkyhold_wd_app::cache_options(),
            )?;
            println!(
                "Backup created.\n\nRecovery key (keep it away from this computer; it is shown only once):\n\n  {}\n",
                *recovery
            );
        }
        Command::Backup {
            only,
            leave_out,
            presets,
            patterns,
        } => {
            let mut settings = Settings::load()?;
            let (target, configured) = target(cli, &settings)?;
            let (dest, repo_path) =
                xkyhold_wd_app::destination(&target, &settings, cli.limit_upload)?;
            let mut repo = open_dest(cli, dest.clone())?;
            let text = |paths: &[PathBuf]| {
                paths
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
            };
            let flags = What {
                only: text(only),
                leave_out: text(leave_out),
                presets: presets.clone(),
                patterns: patterns.clone(),
                cross_mounts: Vec::new(),
            };
            // With no choices on the command line, the configured backup
            // keeps what was chosen in the window, and its retention.
            let use_settings = configured && flags == What::default();
            let what = if use_settings {
                settings.what.clone()
            } else {
                flags
            };
            let keep = if use_settings {
                settings.keep_months
            } else {
                0
            };
            let started = std::time::Instant::now();
            let done = xkyhold_wd_app::run_backup(
                &mut repo,
                dest,
                repo_path.as_deref(),
                &what,
                keep,
                None,
                &mut ticker(),
                &mut || eprintln!("  removing old backups…"),
            )?;
            let snap = &done.snapshot;
            let s = &snap.stats;
            if configured {
                settings.last_backup = Some(LastBackup {
                    time: snap.time.clone(),
                    files: s.files,
                    bytes: s.bytes,
                    skipped: s.skipped,
                });
                settings.save()?;
            }
            println!(
                "Backup {} done in {} s: {} files ({}), {} sent.",
                &snap.id[..8],
                started.elapsed().as_secs(),
                s.files,
                human(s.bytes),
                human(done.uploaded)
            );
            for (count, what) in [
                (s.skipped, "could not be read and were skipped"),
                (
                    s.changed.unwrap_or(0),
                    "changed while being read (stored as read)",
                ),
                (
                    s.locked.unwrap_or(0),
                    "were in use by another program and were not stored (close it and back up again)",
                ),
                (
                    s.cloud_only.unwrap_or(0),
                    "are online-only in OneDrive or another cloud and were not downloaded",
                ),
                (
                    s.bad_names.unwrap_or(0),
                    "have names Windows reports as invalid and were not stored",
                ),
                (
                    s.special.unwrap_or(0),
                    "special items (app aliases and other reparse points) are never stored",
                ),
                (done.forgotten as u64, "old backups were removed"),
            ] {
                if count > 0 {
                    println!("  {count} {what}");
                }
            }
            if !done.issues.is_empty() {
                let list = xkyhold_wd_app::save_issues(&snap.time, &done.issues)?;
                println!("  The items not stored are listed in {}", list.display());
            }
            if let Some(e) = done.cleanup_error {
                eprintln!(
                    "Old backups could not be removed this time ({e}); the new backup is safe."
                );
            }
        }
        Command::Snapshots => {
            let repo = open(cli)?;
            for s in repo.list_snapshots()? {
                let roots: Vec<&str> = s.roots.iter().map(|r| r.name.as_str()).collect();
                println!(
                    "{}  {}  {:12}  {:>8} files  {:>9}  {}",
                    &s.id[..8],
                    s.time,
                    s.device,
                    s.stats.files,
                    human(s.stats.bytes),
                    roots.join(", ")
                );
            }
        }
        Command::Ls { snapshot, path } => {
            let repo = open(cli)?;
            let snap = xkyhold_wd_app::pick(&repo, snapshot)?;
            for e in root_and_entries(&repo, &snap, path.as_deref())? {
                let (kind, size, extra) = match &e.kind {
                    EntryKind::File { size, .. } => ("-", human(*size), String::new()),
                    EntryKind::Dir { mount: true, .. } => {
                        ("d", String::new(), "  (another disk, not backed up)".into())
                    }
                    EntryKind::Dir { .. } => ("d", String::new(), String::new()),
                    EntryKind::Symlink { target, link } => {
                        let kind = link
                            .as_deref()
                            .map(|l| format!(" ({l})"))
                            .unwrap_or_default();
                        (
                            "l",
                            String::new(),
                            format!(" -> {}{kind}", String::from_utf8_lossy(target)),
                        )
                    }
                    EntryKind::Unknown(t) => ("?", String::new(), format!("  (unknown type {t})")),
                };
                let attrs = e.meta.win_attrs.as_ref().map(|a| {
                    let has = |n: &str| a.iter().any(|x| x == n);
                    format!(
                        "{}{}{}",
                        if has("readonly") { 'R' } else { '-' },
                        if has("hidden") { 'H' } else { '-' },
                        if has("system") { 'S' } else { '-' }
                    )
                });
                let attrs = attrs.unwrap_or_else(|| "   ".into());
                println!("{kind} {attrs} {size:>9}  {}{extra}", e.display_name());
            }
        }
        Command::Restore {
            snapshot,
            paths,
            to,
            conflict,
        } => {
            let target = std::path::absolute(to)?;
            let repo = open(cli)?;
            let snap = xkyhold_wd_app::pick(&repo, snapshot)?;
            let mut entries = Vec::new();
            if paths.is_empty() {
                entries = root_and_entries(&repo, &snap, None)?;
            } else {
                for p in paths {
                    let names = split_path(p);
                    let (parent, last) = names.split_at(names.len().saturating_sub(1));
                    let parent = String::from_utf8_lossy(&parent.join(&b'/')).into_owned();
                    let listing = root_and_entries(
                        &repo,
                        &snap,
                        (!parent.is_empty()).then_some(parent.as_str()),
                    )?;
                    let entry = listing
                        .into_iter()
                        .find(|e| last.first().is_some_and(|l| e.name == *l));
                    entries.push(entry.ok_or_else(|| {
                        Error::Usage(format!("nothing at \"{p}\" in this backup"))
                    })?);
                }
            }
            std::fs::create_dir_all(&target)?;
            let conflict = match conflict {
                ConflictArg::Skip => Conflict::Skip,
                ConflictArg::Replace => Conflict::Replace,
                ConflictArg::KeepBoth => Conflict::KeepBoth,
            };
            let r = restore(
                &repo,
                &entries,
                &target,
                &RestoreOptions {
                    conflict,
                    cancel: None,
                },
                &mut |_| {},
            )?;
            println!(
                "Restored {} files ({}), {} folders, {} links into {}.",
                r.files,
                human(r.bytes),
                r.folders,
                r.symlinks,
                target.display()
            );
            for (count, what) in [
                (r.existing, "items already existed and were left alone"),
                (
                    r.bad_names,
                    "items had names that cannot be created and were skipped",
                ),
                (
                    r.unknown,
                    "items of a kind this version does not know were skipped",
                ),
            ] {
                if count > 0 {
                    println!("  {count} {what}");
                }
            }
            // Decision D1/D3: say what was renamed and which links Windows refused.
            if !r.renamed.is_empty() {
                println!(
                    "  {} items were restored under another name, because Windows cannot use theirs:",
                    r.renamed.len()
                );
                for (from, to) in &r.renamed {
                    println!("    {from}  ->  {to}");
                }
            }
            if !r.links_refused.is_empty() {
                println!(
                    "  {} links were not restored: Windows only lets you create them with Developer Mode on \
                     (Settings > System > For developers). Turn it on and restore again to get them back:",
                    r.links_refused.len()
                );
                for (path, target) in &r.links_refused {
                    println!("    {path}  ->  {target}");
                }
            }
        }
        Command::Check { full } => {
            let repo = open(cli)?;
            let report = check(&repo, *full, None, &mut |_| {})?;
            if report.ok() {
                println!(
                    "The backup is complete: {} packs, {} pieces of data checked.",
                    report.packs, report.blobs_checked
                );
            } else {
                println!("Problems found: {report:?}");
                return Err(Error::Authentication("the backup is damaged"));
            }
        }
        Command::Forget { keep_months } => {
            let (dest, path) = destination(cli)?;
            let mut repo = open_dest(cli, dest.clone())?;
            let n = xkyhold_wd_app::forget(&mut repo, dest, path.as_deref(), *keep_months)?;
            println!("{n} old backups removed.");
        }
        Command::Status => {
            let repo = open(cli)?;
            let snapshots = repo.list_snapshots()?;
            println!(
                "Format {}{}; {} backups.",
                repo.config.format,
                if repo.writable() {
                    ""
                } else {
                    " (made by XkyHold for the browser: restore only)"
                },
                snapshots.len()
            );
            if let Some(last) = snapshots.last() {
                println!(
                    "Last: {} from {}, {} files ({}).",
                    last.time,
                    last.device,
                    last.stats.files,
                    human(last.stats.bytes)
                );
                if let Some(rules) = last.roots.first().and_then(|r| r.left_out.as_ref()) {
                    let left: Vec<String> = rules
                        .iter()
                        .map(|r| {
                            let what = r
                                .path
                                .clone()
                                .or_else(|| r.path_bytes.as_deref().map(|b| format!("bytes:{b}")));
                            format!(
                                "{}:{}",
                                r.kind,
                                r.id.clone()
                                    .or(what)
                                    .or_else(|| r.patterns.as_ref().map(|p| p.join(",")))
                                    .unwrap_or_default()
                            )
                        })
                        .collect();
                    println!("Left out: {}", left.join("  "));
                }
            }
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xkyhold-wd-cli: {err}");
            ExitCode::FAILURE
        }
    }
}
