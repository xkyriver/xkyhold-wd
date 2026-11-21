//! Looking at the user folder for the "what to save" screens: the folders
//! inside a folder and the space each suggested exclusion would save.
//! Never follows symbolic links, junctions or mount points (they are
//! reparse points, which `file_type` reports as links), and only reads
//! metadata, so OneDrive online-only files are never downloaded.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use xkyhold_core::rules::{DEFAULTS, PRESETS, glob_match};
use xkyhold_core::{Error, Result};

/// Top-level folders that cloud storage apps keep in sync. They are backed
/// up by default (their online-only files are counted, not read); the
/// window shows them as candidates to leave out. OneDrive for work or
/// school is "OneDrive - <organisation>".
pub const CLOUD_FOLDERS: &[&str] = &[
    "OneDrive",
    "Dropbox",
    "Google Drive",
    "iCloudDrive",
    "MEGA",
    "Nextcloud",
    "ownCloud",
    "pCloud Drive",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub name: String,
    /// The name is valid Unicode. Others cannot be chosen in the settings,
    /// which store text.
    pub text_name: bool,
    /// Never on Windows (kept so the window reads alike on both systems).
    pub mount: bool,
    /// A cloud storage folder.
    pub cloud: bool,
    /// Always left out (temporary files, registry hives, this app's state).
    pub always_out: bool,
}

/// Windows names ignore case: compare folded, as the engine's rules do.
fn matches(pattern: &str, rel: &str) -> bool {
    glob_match(
        pattern.to_ascii_lowercase().as_bytes(),
        rel.to_ascii_lowercase().as_bytes(),
    )
}

fn always_out(rel: &str) -> bool {
    DEFAULTS
        .iter()
        .flat_map(|(_, patterns)| patterns.iter())
        .any(|p| matches(p, rel))
}

fn is_cloud(name: &str) -> bool {
    CLOUD_FOLDERS.iter().any(|c| c.eq_ignore_ascii_case(name))
        || name.to_ascii_lowercase().starts_with("onedrive - ")
}

fn join(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

/// The folders directly inside `rel` (a path inside the user folder; empty
/// for the user folder itself).
pub fn folders(home: &Path, rel: &str) -> Result<Vec<Folder>> {
    let rel_path = String::from_utf8(crate::inside_home(home, Path::new(rel))?)
        .map_err(|_| Error::OutsideHome(rel.into()))?;
    let dir = home.join(&rel_path);
    let read = std::fs::read_dir(&dir).map_err(|e| Error::Read {
        path: dir.display().to_string(),
        source: e,
    })?;
    let mut out = Vec::new();
    for entry in read.flatten() {
        // file_type does not follow reparse points: links and junctions are
        // not folders here.
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let name_os = entry.file_name();
        let name = name_os.to_string_lossy().into_owned();
        let child = join(&rel_path, &name);
        out.push(Folder {
            text_name: name_os.to_str().is_some(),
            mount: false,
            cloud: rel_path.is_empty() && is_cloud(&name),
            always_out: always_out(&child),
            name,
        });
    }
    out.sort_by_key(|f| f.name.to_lowercase());
    Ok(out)
}

/// The space each suggested exclusion would save, by preset id. Walks the
/// whole user folder once (minus what is always left out), so it runs in
/// the background and stops when `cancel` is set.
pub fn preset_sizes(home: &Path, cancel: &AtomicBool) -> Result<HashMap<&'static str, u64>> {
    let mut sizes: HashMap<&'static str, u64> = PRESETS.iter().map(|p| (p.id, 0)).collect();
    let mut stack: Vec<String> = vec![String::new()];
    while let Some(rel) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Aborted);
        }
        let dir = home.join(&rel);
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue;
            }
            let child = join(&rel, &entry.file_name().to_string_lossy());
            if always_out(&child) {
                continue;
            }
            let preset = PRESETS.iter().find(|p| {
                p.patterns.iter().any(|g| matches(g, &child))
                    && p.beside.is_none_or(|file| {
                        dir.join(file).symlink_metadata().is_ok_and(|m| m.is_file())
                    })
            });
            if let Some(preset) = preset {
                let size = if kind.is_dir() {
                    tree_size(&entry.path(), cancel)
                } else {
                    entry.metadata().map_or(0, |m| m.len())
                };
                *sizes.get_mut(preset.id).expect("every preset") += size;
            } else if kind.is_dir() {
                stack.push(child);
            }
        }
    }
    Ok(sizes)
}

/// Bytes in regular files below `dir`, without crossing reparse points.
fn tree_size(dir: &Path, cancel: &AtomicBool) -> u64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let Ok(read) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in read.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                total += entry.metadata().map_or(0, |m| m.len());
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn lists_folders_and_sizes_presets() {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        for d in [
            "Documents",
            "OneDrive - Example",
            "AppData/Local/Temp/x",
            "code/app/node_modules/lib",
            "Downloads",
        ] {
            std::fs::create_dir_all(h.join(d)).unwrap();
        }
        std::fs::write(h.join("code/app/node_modules/lib/a.js"), vec![0u8; 1000]).unwrap();
        std::fs::write(h.join("Downloads/big.iso"), vec![0u8; 500]).unwrap();
        std::fs::create_dir_all(h.join("code/rs/target/debug")).unwrap();
        std::fs::write(h.join("code/rs/Cargo.toml"), b"[package]").unwrap();
        std::fs::write(h.join("code/rs/target/debug/app.exe"), vec![0u8; 300]).unwrap();
        std::fs::create_dir_all(h.join("Photos/target")).unwrap();
        std::fs::write(h.join("Photos/target/beach.jpg"), vec![0u8; 70]).unwrap();
        // A junction is never followed.
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(h.join("link"))
            .arg(h.join("Downloads"))
            .output()
            .unwrap();
        assert!(made.status.success());

        let top = folders(h, "").unwrap();
        let names: Vec<&str> = top.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "AppData",
                "code",
                "Documents",
                "Downloads",
                "OneDrive - Example",
                "Photos"
            ]
        );
        assert!(top[4].cloud);
        assert!(!top[2].cloud && !top[2].always_out);
        let local = folders(h, "AppData/Local").unwrap();
        assert!(local.iter().find(|f| f.name == "Temp").unwrap().always_out);
        assert!(folders(h, "../").is_err());

        let sizes = preset_sizes(h, &AtomicBool::new(false)).unwrap();
        assert_eq!(sizes["node-modules"], 1000);
        assert_eq!(sizes["downloads"], 500);
        assert_eq!(sizes["rust-target"], 300);
    }

    #[test]
    fn cloud_names() {
        assert!(is_cloud("OneDrive"));
        assert!(is_cloud("OneDrive - Contoso"));
        assert!(is_cloud("dropbox"));
        assert!(!is_cloud("Documents"));
    }

    #[cfg(windows)]
    #[test]
    fn always_out_on_windows() {
        assert!(always_out("AppData/Local/Temp"));
        assert!(always_out("ntuser.dat.LOG1"));
        assert!(!always_out("Documents"));
    }
}
