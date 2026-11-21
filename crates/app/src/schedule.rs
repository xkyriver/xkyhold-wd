//! Automatic backups and reminders while the window is closed (PLAN.md 4.8,
//! decision W7).
//!
//! One Task Scheduler task of the current user runs `xkyhold-wd.exe
//! --scheduled` every hour and a little after logon. Each run decides from
//! the settings: an automatic backup when one is due, else a reminder when
//! one is due, else nothing. Automatic backups are opt-in; reminders are the
//! default, as in XkyHold-lx.
//!
//! The task runs only while the user is signed in, with the lowest
//! privileges, and never asks for elevation (W2). It is registered with
//! `schtasks.exe` from an XML definition, so turning everything off removes
//! it again.

use std::path::{Path, PathBuf};
use std::process::Command;

use xkyhold_core::time::{iso, now_ms, parse_iso};
use xkyhold_core::{Error, Result};

use crate::{BackupDone, LastBackup, Settings, human, not_stored};

/// The task's names for the user `user`: in its own folder, and at the top
/// level when Windows does not let this user make or use the folder. Task
/// names belong to the whole computer, so each user's task carries the user
/// name (two people on one computer each have their own).
pub fn task_names(user: &str) -> [String; 2] {
    // Characters a task name cannot hold become `_` (Windows user names
    // rarely have them).
    let user: String = user
        .chars()
        .map(|c| {
            if c < ' ' || "\\/:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    [
        format!(r"\XkyHold\xkyhold-wd-{user}"),
        format!(r"\XkyHold-wd-{user}"),
    ]
}

/// This user's task names.
fn names() -> [String; 2] {
    task_names(&std::env::var("USERNAME").unwrap_or_else(|_| "user".into()))
}

/// The name toasts show, registered for this user (`HKCU`).
pub const APP_ID: &str = "com.xkyriver.xkyhold-wd";

const HOUR: i64 = 3_600_000;
const DAY: i64 = 24 * HOUR;

/// What the hourly run should do.
#[derive(Debug, PartialEq, Eq)]
pub enum Due {
    Backup,
    Remind,
    Nothing,
}

/// The time between automatic backups, a little short so that an hourly
/// task does not drift a whole hour later each time.
fn interval(automatic: &str) -> Option<i64> {
    match automatic {
        "hourly" => Some(HOUR - 5 * 60_000),
        "daily" => Some(DAY - HOUR),
        "weekly" => Some(7 * DAY - HOUR),
        _ => None,
    }
}

fn since(time: Option<&str>, now: i64) -> Option<i64> {
    time.and_then(parse_iso).map(|t| now - t)
}

pub fn due(s: &Settings, now: i64) -> Due {
    if s.backup.is_none() {
        return Due::Nothing;
    }
    let last = since(s.last_backup.as_ref().map(|l| l.time.as_str()), now);
    if let Some(every) = interval(&s.automatic) {
        return if last.is_none_or(|l| l >= every) {
            Due::Backup
        } else {
            Due::Nothing
        };
    }
    if s.remind_days == 0 {
        return Due::Nothing;
    }
    let overdue = last.is_none_or(|l| l >= i64::from(s.remind_days) * DAY - HOUR);
    let quiet = since(s.last_reminded.as_deref(), now).is_some_and(|r| r < 20 * HOUR);
    if overdue && !quiet {
        Due::Remind
    } else {
        Due::Nothing
    }
}

/// The task is wanted while a backup is set up and automatic backups or
/// reminders are on.
pub fn wanted(s: &Settings) -> bool {
    s.backup.is_some() && (interval(&s.automatic).is_some() || s.remind_days > 0)
}

fn portuguese(s: &Settings) -> bool {
    match s.language.as_deref() {
        Some(l) => l == "pt-PT",
        // The Windows display language, as the window's web view sees it.
        None => std::env::var("XKYHOLD_WD_LANG")
            .ok()
            .or_else(windows_language)
            .is_some_and(|l| l.to_ascii_lowercase().starts_with("pt")),
    }
}

/// The user's display language, from the registry (`pt-PT`, `en-US`…).
fn windows_language() -> Option<String> {
    let out = hidden(Command::new("reg").args([
        "query",
        r"HKCU\Control Panel\International",
        "/v",
        "LocaleName",
    ]))
    .output()
    .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find(|l| l.contains("LocaleName"))
        .and_then(|l| l.split_whitespace().last())
        .map(String::from)
}

/// No console window flashes when the hidden task starts a helper.
fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// `<`, `>`, `&`, `'` and `"` for XML text and attributes.
fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// The PowerShell script that shows one toast, under `app_id`.
pub fn toast_script(app_id: &str, title: &str, body: &str) -> String {
    let toast = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        xml(title),
        xml(body)
    );
    // Single quotes in PowerShell strings are doubled.
    let q = |s: &str| s.replace('\'', "''");
    format!(
        "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null\n\
         [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null\n\
         $x = New-Object Windows.Data.Xml.Dom.XmlDocument\n\
         $x.LoadXml('{}')\n\
         [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('{}').Show([Windows.UI.Notifications.ToastNotification]::new($x))\n",
        q(&toast),
        q(app_id)
    )
}

/// A Windows notification (toast). One that cannot be shown is logged.
fn notify(title: &str, body: &str) {
    let script = toast_script(APP_ID, title, body);
    // -EncodedCommand takes UTF-16LE in base64: no quoting problems.
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let shown = hidden(Command::new("powershell.exe").args([
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-EncodedCommand",
        &xkyhold_core::bytes::to_base64(&utf16),
    ]))
    .status();
    if !shown.is_ok_and(|s| s.success()) {
        crate::log(&format!("notification not shown: {title}: {body}"));
    }
}

fn plural(n: i64, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

fn remind(s: &Settings, now: i64) {
    let pt = portuguese(s);
    let days = since(s.last_backup.as_ref().map(|l| l.time.as_str()), now).map(|l| l / DAY);
    let body = match (days, pt) {
        (None, false) => "You have not made a backup yet. Open XkyHold-wd to make one.".to_owned(),
        (None, true) => {
            "Ainda não fizeste nenhuma cópia de segurança. Abre o XkyHold-wd para fazer uma."
                .to_owned()
        }
        (Some(d), false) => format!(
            "Your last backup was {} ago. Open XkyHold-wd to make one.",
            plural(d, "day", "days")
        ),
        (Some(d), true) => format!(
            "A última cópia de segurança foi há {}. Abre o XkyHold-wd para fazer uma.",
            plural(d, "dia", "dias")
        ),
    };
    notify(
        if pt {
            "Está na altura de fazer uma cópia"
        } else {
            "Time for a backup"
        },
        &body,
    );
}

const NEEDS_KEY: &str = "the backup's key is not in the Credential Manager";

/// The sentence for an automatic backup that could not run.
fn problem_text(e: &Error, pt: bool) -> &'static str {
    match (e, pt) {
        (Error::Usage(m), false) if m == NEEDS_KEY => {
            "XkyHold-wd needs to keep your backup's key on this computer. In Settings, tick “Open this backup on this computer without the password”."
        }
        (Error::Usage(m), true) if m == NEEDS_KEY => {
            "O XkyHold-wd precisa de guardar a chave da cópia neste computador. Nas Definições, assinala «Abrir esta cópia neste computador sem a palavra-passe»."
        }
        (Error::Session(_), false) => {
            "Your Google session expired. Open XkyHold-wd and sign in again."
        }
        (Error::Session(_), true) => {
            "A tua sessão Google expirou. Abre o XkyHold-wd e entra de novo."
        }
        (Error::NotFound(m), false) if m.starts_with("Google Drive backup") => {
            "Your backup is no longer in your Google Drive. Open XkyHold-wd to see what to do."
        }
        (Error::NotFound(m), true) if m.starts_with("Google Drive backup") => {
            "A tua cópia de segurança já não está no Google Drive. Abre o XkyHold-wd para ver o que fazer."
        }
        (Error::Io(_) | Error::NotFound(_), false) => {
            "The backup could not be reached. If it is on a disk, connect it: the next automatic backup runs by itself."
        }
        (Error::Io(_) | Error::NotFound(_), true) => {
            "Não foi possível chegar à cópia de segurança. Se estiver num disco, liga-o: a próxima cópia automática corre sozinha."
        }
        (Error::Network(_), false) => {
            "Google Drive could not be reached for a while. Check the connection."
        }
        (Error::Network(_), true) => {
            "Há algum tempo que não é possível contactar o Google Drive. Verifica a ligação."
        }
        (_, false) => "Open XkyHold-wd and press “Back up now” to see what is wrong.",
        (_, true) => "Abre o XkyHold-wd e carrega em «Fazer cópia agora» para ver o que se passa.",
    }
}

/// Whether a problem notice may be shown now (at most once a day), and
/// records that it was.
fn may_tell_problem(s: &Settings, now: i64) -> bool {
    if since(s.last_problem.as_deref(), now).is_some_and(|p| p < 20 * HOUR) {
        return false;
    }
    let stamp = iso(now);
    let _ = Settings::update(|s| {
        s.last_problem = Some(stamp);
        Ok(())
    });
    true
}

/// Problems are shown at most once a day. A network that is down is only
/// worth a message when backups have been failing for two days.
fn report_problem(e: &Error, s: &Settings, now: i64) {
    crate::log(&format!("automatic backup failed: {} ({e})", e.kind()));
    let last = since(s.last_backup.as_ref().map(|l| l.time.as_str()), now);
    if matches!(e, Error::Network(_)) && last.is_some_and(|l| l < 2 * DAY) {
        return;
    }
    if !may_tell_problem(s, now) {
        return;
    }
    let pt = portuguese(s);
    notify(
        if pt {
            "A cópia automática não correu"
        } else {
            "The automatic backup could not run"
        },
        problem_text(e, pt),
    );
}

/// Files that could not be read, and items not stored (in use, online-only,
/// bad names: decision D2), are told at most once a day: a mailbox held open
/// all day would otherwise notify every hour.
fn report_done(done: &BackupDone, s: &Settings, now: i64) {
    let st = &done.snapshot.stats;
    let pt = portuguese(s);
    let left = not_stored(st);
    crate::log(&format!(
        "automatic backup done: {} files, {} sent, {} unreadable, {left} not stored",
        st.files, done.uploaded, st.skipped
    ));
    let title = if pt {
        "Cópia automática concluída"
    } else {
        "Automatic backup done"
    };
    if st.skipped + left > 0 {
        if !may_tell_problem(s, now) {
            return;
        }
        let n = (st.skipped + left) as i64;
        let body = if pt {
            format!(
                "{} ficaram fora desta cópia (em uso, só online ou ilegíveis). Abre o XkyHold-wd para ver a lista.",
                plural(n, "item", "itens")
            )
        } else {
            format!(
                "{} were left out of this backup (in use, online-only or unreadable). Open XkyHold-wd to see the list.",
                plural(n, "item", "items")
            )
        };
        notify(title, &body);
    } else if s.notify_success {
        let body = if pt {
            format!(
                "{}, {} enviados.",
                plural(st.files as i64, "ficheiro", "ficheiros"),
                human(done.uploaded)
            )
        } else {
            format!(
                "{}, {} sent.",
                plural(st.files as i64, "file", "files"),
                human(done.uploaded)
            )
        };
        notify(title, &body);
    }
}

fn backup_now(s: &Settings) -> Result<BackupDone> {
    let target = s
        .backup
        .clone()
        .ok_or_else(|| Error::Usage("no backup set up".into()))?;
    let (dest, repo_path) = crate::destination(&target, s, None)?;
    let mut repo =
        crate::open_with_keyring(dest.clone())?.ok_or_else(|| Error::Usage(NEEDS_KEY.into()))?;
    let done = crate::run_backup(
        &mut repo,
        dest,
        repo_path.as_deref(),
        &s.what,
        s.keep_months,
        None,
        &mut |_| {},
        &mut || {},
    )?;
    // Decision D2: the list of what was not stored stays on this computer.
    if let Err(e) = crate::save_issues(&done.snapshot.time, &done.issues) {
        crate::log(&format!(
            "the list of items not stored could not be written: {e}"
        ));
    }
    let st = &done.snapshot.stats;
    let last = LastBackup {
        time: done.snapshot.time.clone(),
        files: st.files,
        bytes: st.bytes,
        skipped: st.skipped,
        not_stored: not_stored(st),
    };
    Settings::update(|s| {
        s.last_backup = Some(last);
        Ok(())
    })?;
    Ok(done)
}

/// The hourly run. Returns an error only for the log; the user hears about
/// problems through a notification.
pub fn run_scheduled() -> Result<()> {
    let s = Settings::load()?;
    let now = now_ms();
    match due(&s, now) {
        Due::Nothing => Ok(()),
        Due::Remind => {
            // Right after setting up there is no backup yet, and the user is
            // about to make one: start counting instead of reminding now.
            let just_set_up = s.last_backup.is_none() && s.last_reminded.is_none();
            if !just_set_up {
                remind(&s, now);
            }
            let stamp = iso(now);
            Settings::update(|s| {
                s.last_reminded = Some(stamp);
                Ok(())
            })?;
            Ok(())
        }
        Due::Backup => match backup_now(&s) {
            Ok(done) => {
                report_done(&done, &s, now);
                Ok(())
            }
            // The window or the command line is backing up right now.
            Err(Error::RunningHere) => Ok(()),
            Err(e) => {
                report_problem(&e, &s, now);
                Err(e)
            }
        },
    }
}

// --- The scheduled task ------------------------------------------------------

/// The task definition (Task Scheduler schema 1.4): hourly with a random
/// delay and ten minutes after logon, as soon as possible after a missed
/// start, only while the user is signed in, lowest privileges, on battery
/// too, without a time limit (a first backup can take days), never two at
/// once, at below-normal priority.
pub fn task_xml(program: &Path, start: &str) -> String {
    let program = program.display().to_string();
    // The folder holding the program (split by hand: the same on any host).
    let dir = program
        .rsplit_once(['\\', '/'])
        .map(|(dir, _)| dir.to_owned())
        .unwrap_or_default();
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Author>XkyHold-wd</Author>
    <Description>XkyHold-wd: every hour, an automatic backup or a reminder when one is due. Changed or removed from XkyHold-wd's Settings.</Description>
  </RegistrationInfo>
  <Triggers>
    <TimeTrigger>
      <StartBoundary>{start}</StartBoundary>
      <Enabled>true</Enabled>
      <Repetition>
        <Interval>PT1H</Interval>
        <StopAtDurationEnd>false</StopAtDurationEnd>
      </Repetition>
      <RandomDelay>PT5M</RandomDelay>
    </TimeTrigger>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
      <Delay>PT10M</Delay>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{command}</Command>
      <Arguments>--scheduled</Arguments>
      <WorkingDirectory>{dir}</WorkingDirectory>
    </Exec>
  </Actions>
</Task>
"#,
        start = xml(start),
        user = xml(&user()),
        command = xml(&program),
        dir = xml(&dir),
    )
}

/// `DOMAIN\user` of whoever runs this.
fn user() -> String {
    match (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
        (Ok(d), Ok(u)) => format!("{d}\\{u}"),
        (_, Ok(u)) => u,
        _ => String::new(),
    }
}

fn schtasks(args: &[&str]) -> Result<std::process::Output> {
    hidden(Command::new("schtasks.exe").args(args))
        .output()
        .map_err(|e| Error::Usage(format!("Task Scheduler: schtasks could not run: {e}")))
}

fn exists(name: &str) -> bool {
    schtasks(&["/Query", "/TN", name]).is_ok_and(|o| o.status.success())
}

/// The name this user's task is registered under, if it is.
pub fn installed() -> Option<String> {
    names().into_iter().find(|n| exists(n))
}

/// The program the task runs: this one.
fn program() -> Result<PathBuf> {
    Ok(std::env::current_exe()?)
}

/// Registers the toast name for this user, so notifications say
/// "XkyHold-wd". Only `HKCU`; nothing needs elevation.
fn register_app_id() {
    let key = format!(r"HKCU\Software\Classes\AppUserModelId\{APP_ID}");
    let done = hidden(Command::new("reg").args([
        "add",
        &key,
        "/v",
        "DisplayName",
        "/t",
        "REG_SZ",
        "/d",
        "XkyHold-wd",
        "/f",
    ]))
    .output();
    if !done.is_ok_and(|o| o.status.success()) {
        crate::log("the notification name could not be registered");
    }
}

fn unregister_app_id() {
    let key = format!(r"HKCU\Software\Classes\AppUserModelId\{APP_ID}");
    let _ = hidden(Command::new("reg").args(["delete", &key, "/f"])).output();
}

fn remove_task() {
    for name in names() {
        if exists(&name) {
            let _ = schtasks(&["/Delete", "/TN", &name, "/F"]);
        }
    }
}

/// Writes the XML as UTF-16 with a byte order mark, as schtasks expects.
fn write_utf16(path: &Path, text: &str) -> Result<()> {
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Installs, updates or removes the task to match the settings: it is there
/// while a backup is set up and automatic backups or reminders are on. Also
/// keeps it pointing at this program, which may have moved.
pub fn sync_task(s: &Settings) -> Result<()> {
    if !wanted(s) {
        remove_task();
        unregister_app_id();
        return Ok(());
    }
    register_app_id();
    let dir = crate::app_dir("LOCALAPPDATA")?;
    std::fs::create_dir_all(&dir)?;
    let file = dir.join("task.xml");
    // Starts counting from the next whole hour.
    // In UTC, said so with `Z`: without it Windows reads local time.
    let start = iso(now_ms() - now_ms() % HOUR + HOUR);
    let start = format!("{}Z", start.get(..19).unwrap_or(&start));
    write_utf16(&file, &task_xml(&program()?, &start))?;
    let file_arg = file.display().to_string();
    let create = |name: &str| schtasks(&["/Create", "/TN", name, "/XML", &file_arg, "/F"]);
    let [task, fallback] = names();
    let out = create(&task)?;
    let result = if out.status.success() {
        // Only one of the two names at a time.
        if exists(&fallback) {
            let _ = schtasks(&["/Delete", "/TN", &fallback, "/F"]);
        }
        crate::log(&format!(
            "scheduled task {task} registered, hourly from {start}"
        ));
        Ok(())
    } else {
        let first = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        let out = create(&fallback)?;
        if out.status.success() {
            crate::log(&format!("task registered as {fallback}: {first}"));
            Ok(())
        } else {
            Err(Error::Usage(format!(
                "Task Scheduler: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )))
        }
    };
    let _ = std::fs::remove_file(&file);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Target;

    fn settings(automatic: &str, last_hours_ago: Option<i64>, now: i64) -> Settings {
        Settings {
            backup: Some(Target::Local {
                path: r"E:\XkyHold".into(),
            }),
            automatic: automatic.into(),
            last_backup: last_hours_ago.map(|h| LastBackup {
                time: iso(now - h * HOUR),
                files: 1,
                bytes: 1,
                skipped: 0,
                not_stored: 0,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn decides_what_is_due() {
        let now = parse_iso("2026-11-21T12:00:00.000Z").unwrap();
        assert_eq!(due(&settings("daily", None, now), now), Due::Backup);
        assert_eq!(due(&settings("daily", Some(23), now), now), Due::Backup);
        assert_eq!(due(&settings("daily", Some(22), now), now), Due::Nothing);
        assert_eq!(due(&settings("hourly", Some(1), now), now), Due::Backup);
        assert_eq!(due(&settings("weekly", Some(100), now), now), Due::Nothing);
        // Automatic off: the reminder, at most once a day.
        let mut s = settings("off", Some(30), now);
        assert_eq!(due(&s, now), Due::Remind);
        s.last_reminded = Some(iso(now - 2 * HOUR));
        assert_eq!(due(&s, now), Due::Nothing);
        s.last_reminded = Some(iso(now - 21 * HOUR));
        assert_eq!(due(&s, now), Due::Remind);
        s.remind_days = 7;
        assert_eq!(due(&s, now), Due::Nothing);
        s.remind_days = 0;
        assert_eq!(due(&s, now), Due::Nothing);
        assert!(!wanted(&s));
        // Nothing set up: nothing to do, and no task.
        assert_eq!(due(&Settings::default(), now), Due::Nothing);
        assert!(!wanted(&Settings::default()));
    }

    #[test]
    fn the_task_definition() {
        let x = task_xml(
            Path::new(r"C:\Users\Ana & Co\AppData\Local\XkyHold-wd\xkyhold-wd.exe"),
            "2026-11-21T13:00:00Z",
        );
        assert!(
            x.contains(
                r"<Command>C:\Users\Ana &amp; Co\AppData\Local\XkyHold-wd\xkyhold-wd.exe</Command>"
            ),
            "{x}"
        );
        assert!(x.contains(
            r"<WorkingDirectory>C:\Users\Ana &amp; Co\AppData\Local\XkyHold-wd</WorkingDirectory>"
        ));
        assert!(x.contains("<Arguments>--scheduled</Arguments>"));
        for must in [
            "<LogonType>InteractiveToken</LogonType>",
            "<RunLevel>LeastPrivilege</RunLevel>",
            "<StartWhenAvailable>true</StartWhenAvailable>",
            "<Interval>PT1H</Interval>",
            "<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>",
            "<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>",
            "<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>",
            "<StartBoundary>2026-11-21T13:00:00Z</StartBoundary>",
        ] {
            assert!(x.contains(must), "{must}");
        }
    }

    #[test]
    fn one_task_per_user() {
        assert_eq!(
            task_names("ana"),
            [r"\XkyHold\xkyhold-wd-ana", r"\XkyHold-wd-ana"]
        );
        assert_eq!(
            task_names("João Silva")[0],
            r"\XkyHold\xkyhold-wd-João Silva"
        );
        assert_eq!(task_names("a/b:c")[1], r"\XkyHold-wd-a_b_c");
    }

    #[test]
    fn toast_text_is_escaped() {
        let s = toast_script(APP_ID, "A & B", "it's <ok>");
        assert!(s.contains("<text>A &amp; B</text>"), "{s}");
        // The apostrophe is an XML entity, so the PowerShell string stays closed.
        assert!(s.contains("<text>it&apos;s &lt;ok&gt;</text>"), "{s}");
        assert!(s.contains("CreateToastNotifier('com.xkyriver.xkyhold-wd')"));
    }
}
