//! `%APPDATA%\xkyhold-wd\config.toml`: everything about this
//! computer's backup except secrets (PLAN.md 4.5).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use xkyhold_core::{Error, Result};

use crate::app_dir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Target {
    /// The folder `XkyHold/<name>` in the connected Google Drive.
    Drive { name: String },
    /// A folder on this computer, an external disk or a network share.
    Local { path: PathBuf },
}

/// What goes into the backup. Paths are relative to the home folder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct What {
    /// Only these items; empty means the whole home folder.
    pub only: Vec<String>,
    pub leave_out: Vec<String>,
    /// Ids of suggested exclusions switched on.
    pub presets: Vec<String>,
    pub patterns: Vec<String>,
    /// Not used on Windows (kept so settings files read alike).
    pub cross_mounts: Vec<String>,
}

/// The last backup made from this computer, shown before the backup opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastBackup {
    pub time: String,
    pub files: u64,
    pub bytes: u64,
    #[serde(default)]
    pub skipped: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The Google account connected on this computer.
    pub account: Option<String>,
    pub backup: Option<Target>,
    /// The repository's id, to find its key in the keyring.
    pub repo_id: Option<String>,
    pub what: What,
    /// Days between reminders; 0 turns them off.
    pub remind_days: u32,
    /// Months of backups to keep; 0 keeps everything.
    pub keep_months: u32,
    /// "en" or "pt-PT"; unset follows the system.
    pub language: Option<String>,
    /// "system", "light" or "dark".
    pub theme: String,
    pub last_backup: Option<LastBackup>,
    /// Automatic backups (Task Scheduler): "off", "hourly", "daily" or "weekly".
    pub automatic: String,
    /// Also notify when an automatic backup goes well (problems always do).
    pub notify_success: bool,
    /// When the last reminder or problem was shown, to show each at most
    /// once a day.
    pub last_reminded: Option<String>,
    pub last_problem: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            account: None,
            backup: None,
            repo_id: None,
            what: What::default(),
            remind_days: 1,
            keep_months: 0,
            language: None,
            theme: "system".into(),
            last_backup: None,
            automatic: "off".into(),
            notify_success: false,
            last_reminded: None,
            last_problem: None,
        }
    }
}

pub fn path() -> Result<PathBuf> {
    Ok(app_dir("APPDATA")?.join("config.toml"))
}

/// Serializes changes within this process: the window saves settings from
/// several threads (a choice in Settings while a backup records its end).
static UPDATING: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl Settings {
    /// Loads, changes and saves the settings as one step.
    pub fn update(change: impl FnOnce(&mut Settings) -> Result<()>) -> Result<Settings> {
        let _guard = UPDATING.lock().unwrap_or_else(|e| e.into_inner());
        let mut settings = Self::load()?;
        change(&mut settings)?;
        settings.save()?;
        Ok(settings)
    }

    pub fn load() -> Result<Self> {
        match std::fs::read_to_string(path()?) {
            Ok(text) => {
                toml::from_str(&text).map_err(|e| Error::Format(format!("config.toml: {e}")))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Writes atomically: a half-written settings file would lose the backup.
    pub fn save(&self) -> Result<()> {
        let path = path()?;
        let dir = path.parent().expect("has a parent");
        std::fs::create_dir_all(dir)?;
        let tmp = dir.join(".config.toml.tmp");
        std::fs::write(
            &tmp,
            toml::to_string(self).map_err(|e| Error::Format(e.to_string()))?,
        )?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_cli_settings_of_phase_3() {
        let s: Settings = toml::from_str("account = \"someone@example.com\"\n").unwrap();
        assert_eq!(s.account.as_deref(), Some("someone@example.com"));
        assert_eq!(s.remind_days, 1);
        assert_eq!(s.theme, "system");
        assert!(s.backup.is_none());
    }

    #[test]
    fn round_trips() {
        let s = Settings {
            backup: Some(Target::Local {
                path: r"E:\XkyHold".into(),
            }),
            repo_id: Some("abc".into()),
            what: What {
                leave_out: vec!["Videos".into()],
                presets: vec!["node-modules".into()],
                ..Default::default()
            },
            keep_months: 6,
            last_backup: Some(LastBackup {
                time: "2026-11-14T10:00:00.000Z".into(),
                files: 3,
                bytes: 9,
                skipped: 0,
            }),
            ..Default::default()
        };
        let text = toml::to_string(&s).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), s);
        let drive = Settings {
            backup: Some(Target::Drive {
                name: "laptop".into(),
            }),
            ..Default::default()
        };
        assert_eq!(
            toml::from_str::<Settings>(&toml::to_string(&drive).unwrap()).unwrap(),
            drive
        );
    }
}
