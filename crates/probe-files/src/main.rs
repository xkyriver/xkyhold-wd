//! Phase 0 probe (risk d). For each path: print the file attributes, then try
//! to open and read the whole file the way the backup will (std's default
//! share mode on Windows is read | write | delete), and report the OS error.
//! The probe never writes. Throwaway: it answers a question in docs/PLAN.md.

use std::fs::File;
use std::io::Read;

const FLAGS: &[(u32, &str)] = &[
    (0x1, "readonly"),
    (0x2, "hidden"),
    (0x4, "system"),
    (0x20, "archive"),
    (0x400, "reparse"),
    (0x1000, "offline"),
    (0x2000, "notIndexed"),
    (0x8_0000, "pinned"),
    (0x10_0000, "unpinned"),
    (0x4_0000, "recallOnOpen"),
    (0x40_0000, "recallOnDataAccess"),
];

#[cfg(windows)]
fn attributes(meta: &std::fs::Metadata) -> u32 {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes()
}

#[cfg(not(windows))]
fn attributes(_: &std::fs::Metadata) -> u32 {
    0
}

fn main() {
    for path in std::env::args().skip(1) {
        print!("{path}\n  ");
        match std::fs::symlink_metadata(&path) {
            Ok(meta) => {
                let a = attributes(&meta);
                let names: Vec<_> = FLAGS
                    .iter()
                    .filter(|(bit, _)| a & bit != 0)
                    .map(|(_, n)| *n)
                    .collect();
                println!(
                    "attributes 0x{a:x} [{}], size {}",
                    names.join(" "),
                    meta.len()
                );
            }
            Err(e) => println!("metadata error: {e} (os {:?})", e.raw_os_error()),
        }
        match File::open(&path) {
            Err(e) => println!("  open: {e} (os {:?})", e.raw_os_error()),
            Ok(mut f) => {
                let mut buf = Vec::new();
                match f.read_to_end(&mut buf) {
                    Ok(n) => println!("  read: ok, {n} bytes"),
                    Err(e) => println!(
                        "  read: {e} (os {:?}) after {} bytes",
                        e.raw_os_error(),
                        buf.len()
                    ),
                }
            }
        }
    }
}
