//! Development builds only: lets a test script drive the window when
//! `XKYHOLD_WD_PROBE` names a folder. Each `<n>.js` dropped there is a
//! function body run in the page; its return value lands in `<n>.json`.
//! Release builds do not contain this module.

use std::path::PathBuf;
use std::time::Duration;

use tauri::{AppHandle, Manager};

pub fn start(app: &AppHandle) {
    let Some(dir) = std::env::var_os("XKYHOLD_WD_PROBE").map(PathBuf::from) else {
        return;
    };
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let Ok(read) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in read.flatten() {
                let path = entry.path();
                if path.extension().is_none_or(|e| e != "js") {
                    continue;
                }
                let Ok(body) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let _ = std::fs::remove_file(&path);
                let out = path.with_extension("json");
                let Some(window) = app.get_webview_window("main") else {
                    continue;
                };
                let script = format!(
                    "(function(){{try{{ {body} \n}}catch(e){{return 'ERR: '+e.message}}}})()"
                );
                let _ = window.eval_with_callback(script, move |result| {
                    let tmp = out.with_extension("tmp");
                    if std::fs::write(&tmp, result).is_ok() {
                        let _ = std::fs::rename(&tmp, &out);
                    }
                });
            }
        }
    });
}
