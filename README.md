# XkyHold-wd

Encrypted backup of your Windows user folder to your own Google Drive or to
a local folder. The Windows member of the
[XkyHold](https://xkyhold.xkyriver.com) family: same encryption, same
repository format and same look, without the limits of a browser.

**Status:** planning. Nothing to download yet.

## Scope

- Only `%USERPROFILE%` and what is inside it, including `AppData`, hidden
  files and `.ssh`. It never runs as administrator and is not a system
  backup.
- Files locked by another program are skipped and listed.
- Repositories written by XkyHold-wd can be restored by XkyHold web and
  XkyHold-lx, and the other way round.
