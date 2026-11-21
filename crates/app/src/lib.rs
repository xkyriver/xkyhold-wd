//! What the window and the CLI share: where things live on this computer,
//! the settings file, opening a backup (Google Drive or a local folder)
//! with the Credential Manager or a password, and running a backup end to
//! end. The Windows counterpart of XkyHold-lx's `xkyhold-app`.

pub mod scan;
pub mod settings;

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use xkyhold_core::backup::{BackupOptions, Issue, IssueKind, Progress, backup};
use xkyhold_core::crypto::Credential;
use xkyhold_core::destination::Destination;
use xkyhold_core::lock::{Lock, LockKind};
use xkyhold_core::prune::{forget_older, prune};
use xkyhold_core::repo::{Options, Repository};
use xkyhold_core::restore::find;
use xkyhold_core::rules::{Exclusions, PRESETS};
use xkyhold_core::snapshot::Snapshot;
use xkyhold_core::time::{now_iso, now_ms};
use xkyhold_core::tree::{Entry, EntryKind};
use xkyhold_core::{Error, Result};
use xkyhold_dest_drive::{Client, Drive, DriveFolder, Http, RefreshingTokens, Ureq};
use xkyhold_dest_local::LocalFolder;
use xkyhold_secrets::{MemorySecrets, SecretStore, SystemKeyring, google_refresh, repo_key};
use zeroize::Zeroizing;

pub use settings::{LastBackup, Settings, Target, What};

pub type Repo = Repository<Arc<dyn Destination>>;

/// Average month, for "keep backups for N months".
pub const MONTH_MS: i64 = 2_629_746_000;

/// The user folder, `%USERPROFILE%`. Only it is ever backed up.
pub fn home() -> Result<PathBuf> {
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or(Error::OutsideHome("%USERPROFILE% is not set".into()))?;
    if !home.is_absolute() || home.parent().is_none() {
        return Err(Error::OutsideHome(home.display().to_string()));
    }
    Ok(home)
}

/// The computer's name, recorded in each backup.
pub fn device() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "windows".into())
}

/// `%<var>%\xkyhold-wd`: `APPDATA` for settings, `LOCALAPPDATA` for the
/// cache, the log and the lock.
pub fn app_dir(var: &str) -> Result<PathBuf> {
    match std::env::var_os(var) {
        Some(dir) if Path::new(&dir).is_absolute() => Ok(PathBuf::from(dir).join("xkyhold-wd")),
        _ => Err(Error::Usage(format!("%{var}% is not set"))),
    }
}

/// Appends a line to `%LOCALAPPDATA%\xkyhold-wd\xkyhold-wd.log` (and
/// stderr). Raw errors go here; people see actionable sentences.
pub fn log(line: &str) {
    let stamped = format!("[{}] {line}", now_iso());
    eprintln!("{stamped}");
    if let Ok(dir) = app_dir("LOCALAPPDATA") {
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("xkyhold-wd.log"))
        {
            let _ = writeln!(f, "{stamped}");
        }
    }
}

/// Where the items one backup could not store are listed (decision D2:
/// the snapshot keeps only the counts).
pub fn issues_path() -> Result<PathBuf> {
    Ok(app_dir("LOCALAPPDATA")?.join("last-backup-not-stored.txt"))
}

/// Writes the list of items the last backup could not store, with why.
pub fn save_issues(snapshot_time: &str, issues: &[Issue]) -> Result<PathBuf> {
    let path = issues_path()?;
    std::fs::create_dir_all(path.parent().expect("has a parent"))?;
    let mut text = format!("Backup of {snapshot_time}: items not stored\r\n");
    for issue in issues {
        let why = match issue.kind {
            IssueKind::Locked => "in use by another program",
            IssueKind::CloudOnly => "online-only (not downloaded)",
            IssueKind::BadName => "name Windows reports as invalid",
        };
        text.push_str(&format!("{}\t{why}\r\n", issue.path));
    }
    std::fs::write(&path, text)?;
    Ok(path)
}

pub fn cache_options() -> Options {
    Options {
        cache: app_dir("LOCALAPPDATA").ok().map(|d| d.join("cache")),
        ..Options::default()
    }
}

pub fn secrets() -> Result<SystemKeyring> {
    SystemKeyring::connect().map_err(credential_manager)
}

/// Errors from the Credential Manager, worded so the window can tell them
/// apart from other usage errors.
fn credential_manager(e: impl std::fmt::Display) -> Error {
    Error::Usage(format!("Credential Manager: {e}"))
}

/// The Google client "XkyHold-wd", built in at compile time from the
/// environment (never in git), or read from it at run time.
pub fn client() -> Result<Client> {
    let id = std::env::var("XKYHOLD_WD_CLIENT_ID")
        .ok()
        .or(option_env!("XKYHOLD_WD_CLIENT_ID").map(String::from));
    let secret = std::env::var("XKYHOLD_WD_CLIENT_SECRET")
        .ok()
        .or(option_env!("XKYHOLD_WD_CLIENT_SECRET").map(String::from));
    match (id, secret) {
        (Some(id), Some(secret)) => Ok(Client { id, secret }),
        _ => Err(Error::Usage(
            "this build has no Google client; set XKYHOLD_WD_CLIENT_ID and XKYHOLD_WD_CLIENT_SECRET".into(),
        )),
    }
}

/// A Drive client for the account connected on this computer.
pub fn drive_client(settings: &Settings, limit_kb: Option<u64>) -> Result<Drive> {
    let account = settings
        .account
        .as_deref()
        .ok_or_else(|| Error::Session("not connected to Google Drive".into()))?;
    let key = google_refresh(account);
    let kept = match secrets() {
        Ok(store) => store.get(&key).map_err(credential_manager)?,
        Err(_) => None,
    };
    let refresh = kept
        .or_else(|| session_secrets().get(&key).ok().flatten())
        .ok_or_else(|| Error::Session("not connected to Google Drive".into()))?;
    let refresh =
        String::from_utf8(refresh).map_err(|_| Error::Session("sign in to Google again".into()))?;
    let http: Arc<dyn Http> = Arc::new(Ureq::default());
    let mut drive = Drive::new(
        http.clone(),
        Arc::new(RefreshingTokens::new(http, client()?, refresh)),
    );
    if let Some(kb) = limit_kb {
        drive.limit_uploads(kb * 1000);
    }
    Ok(drive)
}

/// Opens an address in the default browser.
pub fn open_in_browser(url: &str) -> bool {
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .status()
        .is_ok_and(|s| s.success())
}

/// Signs in to Google in the browser, keeps the refresh token in the
/// Credential Manager (in memory for this run when that fails) and returns
/// the account's email address. `show` gets the sign-in address.
pub fn google_sign_in(show: &mut dyn FnMut(&str)) -> Result<String> {
    google_sign_in_until(show, &AtomicBool::new(false))
}

/// [`google_sign_in`] that stops waiting for the browser once `cancel` is set.
pub fn google_sign_in_until(show: &mut dyn FnMut(&str), cancel: &AtomicBool) -> Result<String> {
    let client = client()?;
    let http = Ureq::default();
    let signed = xkyhold_dest_drive::sign_in_until(&http, &client, show, cancel)?;
    let http: Arc<dyn Http> = Arc::new(http);
    let tokens = Arc::new(RefreshingTokens::new(
        http.clone(),
        client,
        signed.refresh.clone(),
    ));
    let account = Drive::new(http, tokens).about_email()?;
    let key = google_refresh(&account);
    if let Err(e) = secrets().and_then(|s| {
        s.set(&key, signed.refresh.as_bytes())
            .map_err(credential_manager)
    }) {
        log(&format!("Google session kept in memory only: {e}"));
        let _ = session_secrets().set(&key, signed.refresh.as_bytes());
    }
    Ok(account)
}

pub fn google_sign_out(account: &str) -> Result<()> {
    let key = google_refresh(account);
    let _ = session_secrets().delete(&key);
    match secrets() {
        Ok(store) => store.delete(&key).map_err(credential_manager),
        Err(_) => Ok(()),
    }
}

/// Secrets for this run only, when the Credential Manager cannot be used.
fn session_secrets() -> &'static MemorySecrets {
    static SESSION: std::sync::OnceLock<MemorySecrets> = std::sync::OnceLock::new();
    SESSION.get_or_init(MemorySecrets::default)
}

/// The destination of an existing backup, and its local path when it is a
/// folder (left out of the backup when it lies inside the user folder).
/// A Drive backup that is gone (or in the trash) is not recreated.
pub fn destination(
    target: &Target,
    settings: &Settings,
    limit_kb: Option<u64>,
) -> Result<(Arc<dyn Destination>, Option<PathBuf>)> {
    open_destination(target, settings, limit_kb, false)
}

/// The destination for a new backup: its Drive folder is created.
pub fn new_destination(
    target: &Target,
    settings: &Settings,
    limit_kb: Option<u64>,
) -> Result<(Arc<dyn Destination>, Option<PathBuf>)> {
    open_destination(target, settings, limit_kb, true)
}

fn open_destination(
    target: &Target,
    settings: &Settings,
    limit_kb: Option<u64>,
    create: bool,
) -> Result<(Arc<dyn Destination>, Option<PathBuf>)> {
    match target {
        Target::Drive { name } => {
            let drive = drive_client(settings, limit_kb)?;
            let folder = if create {
                DriveFolder::open(drive, name)?
            } else {
                DriveFolder::open_existing(drive, name)?
            };
            folder.warm()?;
            Ok((Arc::new(folder), None))
        }
        Target::Local { path } => {
            let path = std::path::absolute(path)?;
            Ok((Arc::new(LocalFolder::new(&path)), Some(path)))
        }
    }
}

/// What a local folder holds, before using it for a backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderState {
    Missing,
    Empty,
    Repository,
    /// Holds other files: not used, to keep backups apart from other data.
    Other,
}

pub fn inspect_folder(path: &Path) -> FolderState {
    match std::fs::read_dir(path) {
        Err(_) => FolderState::Missing,
        Ok(mut entries) => {
            if path.join("config").is_file() {
                FolderState::Repository
            } else if entries.next().is_none() {
                FolderState::Empty
            } else {
                FolderState::Other
            }
        }
    }
}

/// Opens with the key kept in the Credential Manager, if one fits.
pub fn open_with_keyring(dest: Arc<dyn Destination>) -> Result<Option<Repo>> {
    let config = Repository::read_config(&dest)?;
    let Ok(store) = SystemKeyring::connect() else {
        return Ok(None);
    };
    let Ok(Some(master)) = store.get(&repo_key(&config.repo_id)) else {
        return Ok(None);
    };
    match Repository::open(
        dest,
        &Credential::MasterKey(Zeroizing::new(master)),
        cache_options(),
    ) {
        Err(Error::WrongKey) => Ok(None),
        other => other.map(Some),
    }
}

pub fn open_with(dest: Arc<dyn Destination>, credential: &Credential) -> Result<Repo> {
    Repository::open(dest, credential, cache_options())
}

/// Keeps (or removes) this backup's key in the Credential Manager.
pub fn remember_key(repo: &Repo, remember: bool) -> Result<()> {
    let store = secrets()?;
    let key = repo_key(&repo.config.repo_id);
    let done = if remember {
        store.set(&key, repo.keys.master_key())
    } else {
        store.delete(&key)
    };
    done.map_err(credential_manager)
}

pub fn key_remembered(repo_id: &str) -> bool {
    SystemKeyring::connect()
        .ok()
        .and_then(|s| s.get(&repo_key(repo_id)).ok().flatten())
        .is_some()
}

pub fn forget_key(repo_id: &str) -> Result<()> {
    secrets()?
        .delete(&repo_key(repo_id))
        .map_err(credential_manager)
}

/// A path inside the user folder, relative to it, `/` between names.
/// Refuses anything outside. Windows compares paths without regard to case.
pub fn inside_home(home: &Path, path: &Path) -> Result<Vec<u8>> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        home.join(path)
    };
    let outside = || Error::OutsideHome(path.display().to_string());
    let (abs, base) = (
        absolute.to_string_lossy().to_lowercase(),
        home.to_string_lossy().to_lowercase(),
    );
    let rest = abs.strip_prefix(&base).ok_or_else(outside)?;
    if !(rest.is_empty() || rest.starts_with('\\') || rest.starts_with('/')) {
        return Err(outside());
    }
    // Keep the case as written, after the prefix.
    let tail = &absolute.to_string_lossy()[base.len()..].to_owned();
    let mut parts: Vec<&str> = Vec::new();
    for c in Path::new(tail.trim_start_matches(['\\', '/'])).components() {
        match c {
            std::path::Component::Normal(name) => parts.push(name.to_str().ok_or_else(outside)?),
            std::path::Component::CurDir => {}
            _ => return Err(outside()),
        }
    }
    Ok(parts.join("/").into_bytes())
}

/// The exclusion rules for `what`, with the repository itself left out
/// when it is a folder inside the user folder.
pub fn exclusions(home: &Path, what: &What, repo_path: Option<&Path>) -> Result<Exclusions> {
    let rel = |paths: &[String]| {
        paths
            .iter()
            .map(|p| inside_home(home, Path::new(p)))
            .collect::<Result<Vec<_>>>()
    };
    for id in &what.presets {
        if !PRESETS.iter().any(|p| p.id == id) {
            return Err(Error::Usage(format!("unknown preset \"{id}\"")));
        }
    }
    Ok(Exclusions {
        presets: what.presets.clone(),
        paths: rel(&what.leave_out)?,
        patterns: what.patterns.clone(),
        cross_mounts: Vec::new(),
        repository: repo_path.and_then(|p| inside_home(home, p).ok()),
    })
}

/// One backup at a time on this computer (the window, the scheduled task,
/// the CLI): a file in `%LOCALAPPDATA%\xkyhold-wd` locked while it runs.
pub struct LocalLock(#[allow(dead_code)] File);

impl LocalLock {
    pub fn take() -> Result<Self> {
        let dir = app_dir("LOCALAPPDATA")?;
        std::fs::create_dir_all(&dir)?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join("backup.lock"))?;
        match file.try_lock() {
            Ok(()) => Ok(Self(file)),
            Err(std::fs::TryLockError::WouldBlock) => Err(Error::RunningHere),
            Err(std::fs::TryLockError::Error(e)) => Err(e.into()),
        }
    }
}

/// Items a backup could not store (decision D2): in use by another
/// program, online-only, or with a name Windows reports as invalid.
pub fn not_stored(stats: &xkyhold_core::snapshot::Stats) -> u64 {
    stats.locked.unwrap_or(0) + stats.cloud_only.unwrap_or(0) + stats.bad_names.unwrap_or(0)
}

/// A backup runs on this computer in another process (the scheduled task,
/// the CLI, another window).
pub fn running_here() -> bool {
    matches!(LocalLock::take(), Err(Error::RunningHere))
}

/// Opens a folder in File Explorer, or a file with its default app.
pub fn open_path(path: &Path) -> Result<()> {
    if !path.is_absolute() || !path.exists() {
        return Err(Error::NotFound(path.display().to_string()));
    }
    // explorer.exe returns 1 even when it opened the path: do not wait.
    std::process::Command::new("explorer.exe")
        .arg(path)
        .spawn()?;
    Ok(())
}

/// The outcome of [`run_backup`].
pub struct BackupDone {
    pub snapshot: Snapshot,
    /// Bytes sent by this backup.
    pub uploaded: u64,
    /// Old backups removed afterwards, by the retention setting.
    pub forgotten: usize,
    /// A failure while removing old backups; the new backup is safe.
    pub cleanup_error: Option<Error>,
    /// Items not stored (locked, online-only, bad names), with why.
    pub issues: Vec<Issue>,
}

/// Backs up the user folder by `what`, under the repository lock, then
/// removes backups older than `keep_months` (0 keeps everything).
#[allow(clippy::too_many_arguments)]
pub fn run_backup(
    repo: &mut Repo,
    dest: Arc<dyn Destination>,
    repo_path: Option<&Path>,
    what: &What,
    keep_months: u32,
    cancel: Option<&AtomicBool>,
    on_progress: &mut dyn FnMut(&Progress),
    on_cleanup: &mut dyn FnMut(),
) -> Result<BackupDone> {
    let home = home()?;
    let exclusions = exclusions(&home, what, repo_path)?;
    let selection = if what.only.is_empty() {
        None
    } else {
        Some(
            what.only
                .iter()
                .map(|p| inside_home(&home, Path::new(p)))
                .collect::<Result<Vec<_>>>()?,
        )
    };
    let _here = LocalLock::take()?;
    let device = device();
    let sent_before = repo.uploaded_bytes;
    let mut issues = Vec::new();
    let mut lock = Lock::acquire(dest.clone(), LockKind::Backup, &device)?;
    let result = (|| {
        repo.refresh_index()?;
        let parent = latest_of(repo, &device, &home)?;
        let mut options = BackupOptions::new(device.clone(), now_iso());
        options.parent = parent.as_ref();
        options.exclusions = exclusions;
        options.selection = selection;
        options.cancel = cancel;
        backup(repo, &home, options, &mut |p| {
            if p.issues.len() != issues.len() {
                issues.clone_from(&p.issues);
            }
            on_progress(p)
        })
    })();
    lock.release();
    let snapshot = result?;
    let uploaded = repo.uploaded_bytes - sent_before;
    let (forgotten, cleanup_error) = if keep_months == 0 {
        (0, None)
    } else {
        on_cleanup();
        match forget(repo, dest, repo_path, keep_months) {
            Ok(n) => (n, None),
            Err(e) => (0, Some(e)),
        }
    };
    Ok(BackupDone {
        snapshot,
        uploaded,
        forgotten,
        cleanup_error,
        issues,
    })
}

/// Removes backups older than `keep_months` (always keeps the latest) and
/// frees their space. Returns how many were removed.
pub fn forget(
    repo: &mut Repo,
    dest: Arc<dyn Destination>,
    repo_path: Option<&Path>,
    keep_months: u32,
) -> Result<usize> {
    let since = now_ms() - i64::from(keep_months) * MONTH_MS;
    let mut lock = Lock::acquire(dest, LockKind::Prune, &device())?;
    let result = forget_older(repo, since).and_then(|n| {
        if n > 0 {
            prune(repo, 0.5).map(|_| n)
        } else {
            Ok(0)
        }
    });
    lock.release();
    let n = result?;
    if let Some(path) = repo_path {
        LocalFolder::new(path).empty_old_trash()?;
    }
    Ok(n)
}

/// The listing of a folder inside a backup, given as a path of names.
/// Version 1 backups hold several folders: the first name picks one.
pub fn entries(repo: &Repo, snap: &Snapshot, path: &[&[u8]]) -> Result<Vec<Entry>> {
    let root = snap
        .roots
        .first()
        .ok_or_else(|| Error::Format("backup without folders".into()))?;
    let mut names = path.to_vec();
    let tree = if snap.roots.len() > 1 || root.source.is_none() {
        let first = names.first().copied().unwrap_or_default();
        match snap.roots.iter().find(|r| r.name.as_bytes() == first) {
            Some(r) => {
                names.remove(0);
                r.tree.clone()
            }
            None if names.is_empty() => {
                return Ok(snap
                    .roots
                    .iter()
                    .map(|r| Entry {
                        name: r.name.as_bytes().to_vec(),
                        kind: EntryKind::Dir {
                            subtree: r.tree.clone(),
                            mount: false,
                        },
                        meta: Default::default(),
                    })
                    .collect());
            }
            None => return Err(Error::NotFound(String::from_utf8_lossy(first).into_owned())),
        }
    } else {
        root.tree.clone()
    };
    if names.is_empty() {
        return repo.load_tree(&tree);
    }
    match find(repo, &tree, &names)? {
        Some(Entry {
            kind: EntryKind::Dir { subtree, .. },
            ..
        }) => repo.load_tree(&subtree),
        Some(entry) => Ok(vec![entry]),
        None => Err(Error::NotFound(
            String::from_utf8_lossy(&names.join(&b'/')).into_owned(),
        )),
    }
}

/// The latest backup this computer made of this user folder, if any.
pub fn latest_of_this_computer(repo: &Repo) -> Result<Option<Snapshot>> {
    latest_of(repo, &device(), &home()?)
}

/// By device and user folder (FORMAT-v2-windows.md 3.2): a repository may
/// hold backups of several computers and users.
fn latest_of(repo: &Repo, device: &str, home: &Path) -> Result<Option<Snapshot>> {
    let home = home.to_string_lossy().to_lowercase();
    Ok(repo.list_snapshots()?.into_iter().rev().find(|s| {
        s.device == device
            && s.origin
                .as_ref()
                .is_some_and(|o| o.path.to_lowercase() == home)
    }))
}

/// The choices of what to save, as a backup recorded them, to carry them
/// over when this computer connects to it again.
pub fn what_from_snapshot(snap: &Snapshot) -> What {
    let mut what = What::default();
    // "Only some items" (FORMAT-v2-chosen.md); paths that are not text
    // cannot be chosen in the settings and are left to the user.
    if let Some(chosen) = snap.roots.first().and_then(|r| r.chosen.as_ref()) {
        what.only = chosen.iter().filter_map(|c| c.path.clone()).collect();
    }
    let Some(rules) = snap.roots.first().and_then(|r| r.left_out.as_ref()) else {
        return what;
    };
    for rule in rules {
        match (rule.kind.as_str(), &rule.id, &rule.path) {
            ("preset", Some(id), _) => what.presets.push(id.clone()),
            ("path", _, Some(path)) => what.leave_out.push(path.clone()),
            ("pattern", _, _) => what
                .patterns
                .extend(rule.patterns.clone().unwrap_or_default()),
            _ => {}
        }
    }
    what
}

/// Picks a backup by id, the start of an id, or `latest`.
pub fn pick(repo: &Repo, wanted: &str) -> Result<Snapshot> {
    let snapshots = repo.list_snapshots()?;
    let found = if wanted == "latest" {
        snapshots.last().cloned()
    } else {
        let matching: Vec<_> = snapshots
            .iter()
            .filter(|s| s.id.starts_with(wanted))
            .collect();
        if matching.len() > 1 {
            return Err(Error::Usage(format!(
                "\"{wanted}\" matches more than one backup"
            )));
        }
        matching.first().map(|s| (*s).clone())
    };
    found.ok_or_else(|| Error::NotFound(format!("backup \"{wanted}\"")))
}

/// "1.5 GB", in decimal units like Explorer's details.
pub fn human(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < units.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", units[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn inside_home_refuses_escapes() {
        let home = Path::new(r"C:\Users\ana");
        assert_eq!(
            inside_home(home, Path::new(r"Documents\a")).unwrap(),
            b"Documents/a"
        );
        assert_eq!(
            inside_home(home, Path::new(r"C:\Users\ana\x")).unwrap(),
            b"x"
        );
        assert_eq!(
            inside_home(home, Path::new(r"c:\users\ANA\Desktop")).unwrap(),
            b"Desktop"
        );
        assert!(inside_home(home, Path::new(r"..\bob")).is_err());
        assert!(inside_home(home, Path::new(r"C:\Windows")).is_err());
        assert!(inside_home(home, Path::new(r"C:\Users\anabel")).is_err());
        assert!(inside_home(home, Path::new(r"a\..\..\bob")).is_err());
    }

    #[test]
    fn the_choice_comes_back_from_a_backup() {
        use xkyhold_core::snapshot::{Chosen, Root, Rule, Source};
        let rule = |kind: &str, path: Option<&str>| Rule {
            kind: kind.into(),
            id: None,
            patterns: None,
            path: path.map(String::from),
            path_bytes: None,
        };
        let snap = Snapshot {
            format: Some(2),
            id: "a".into(),
            time: "t".into(),
            device: "d".into(),
            home: None,
            origin: None,
            roots: vec![Root {
                name: "home".into(),
                tree: "x".into(),
                source: Some(Source {
                    kind: "home".into(),
                    path: String::new(),
                }),
                meta: None,
                left_out: Some(vec![rule("path", Some("Documents/Old"))]),
                chosen: Some(vec![
                    Chosen::new(b"Documents"),
                    Chosen::new(b"Desktop"),
                    Chosen::new(b"bad\xff"),
                ]),
            }],
            stats: Default::default(),
            parent: None,
        };
        let what = what_from_snapshot(&snap);
        assert_eq!(what.only, ["Documents", "Desktop"]);
        assert_eq!(what.leave_out, ["Documents/Old"]);
    }

    #[test]
    fn human_sizes() {
        assert_eq!(human(999), "999 B");
        assert_eq!(human(1_500_000), "1.5 MB");
    }
}
