# XkyHold-wd

Windows desktop app in the XkyHold family: a full encrypted backup of
`%USERPROFILE%` (or parts of it) to the user's Google Drive or a local
folder. Plan, decisions W1–W11, phases and state log: `docs/PLAN.md`
(Portuguese). The engine is shared with XkyHold-lx (decision W1): its
crates are made cross-platform on a branch of that repository and this one
depends on them by git revision. The repository format lives in the
XkyHold repository (`docs/FORMAT.md`, `docs/FORMAT-v2.md`); change the
spec first, then the code.

## Hard rules

- What is backed up is `%USERPROFILE%` only: refuse source paths outside
  it, never follow symbolic links, junctions or mount points while backing
  up, never run elevated and never ask for UAC elevation.
- No Volume Shadow Copy. Files locked by another program are skipped and
  counted, never forced.
- Never read OneDrive "online-only" files: reading them makes Windows
  download them.
- Encryption happens on the user's computer; destinations only see
  ciphertext. "Delete" in Drive is always "move to trash".
- Never regenerate `golden` fixtures. A failing golden test means the
  change is wrong.
- Tests use temporary folders, the in-memory destination or the test VM.
  Backup never writes to the source. No restore to the original location
  without an explicit request.
- Automatic backup is opt-in (a per-user Task Scheduler task); the default
  is a reminder.
- Changes to the shared engine must keep XkyHold-lx's tests green on Linux;
  merging or pushing in the XkyHold-lx repository needs the owner's yes.

## Conventions

- Code, comments, commit messages, technical docs: English.
  `docs/PLAN.md` is European Portuguese.
- UI strings: English source, pt-PT translation addressing the user as
  "tu", never "você".
- Commits: a title and bullet points. No local machine paths, personal
  email addresses or employer names in versioned files.
- Keep dependencies few and small.
