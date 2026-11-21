// The window's state: what the engine reported last, the job running, and
// the banners. Screens read `app` and call the functions below.

import { api, on } from '../lib/api'
import type { JobDone, Secret, Settings, Status, UiError } from '../lib/api'
import { formatBytes } from '../lib/format'
import { applyLanguage, fileCount, t } from '../lib/i18n.svelte'

export type View = 'home' | 'restore' | 'settings'

export interface Job {
  kind: 'backup' | 'restore' | 'check'
  files: number
  bytes: number
  current: string
  cleaning: boolean
}

export const app = $state({
  ready: false,
  status: null as Status | null,
  view: 'home' as View,
  /** The configured backup could not be opened with the keyring: ask for the password. */
  needsPassword: false,
  opening: false,
  job: null as Job | null,
  error: null as string | null,
  /** The last error was the Google session running out. */
  expired: false,
  notice: null as string | null,
  /** A folder to offer to open, with the notice. */
  noticeFolder: null as string | null,
  /** Offer to open the list of what the backup could not store (decision D2). */
  noticeIssues: false,
  /** Items to show under the notice: renamed, links not restored (D1, D3). */
  noticeDetails: [] as string[],
  reminderDismissed: false,
  /** The window was asked to close while a job runs. */
  closing: false,
  /** The Google sign-in address, in case the browser did not open. */
  signinUrl: null as string | null,
  /** Bumped after each backup, restore or check, for screens to reload. */
  generation: 0,
  /** The result of the last check, also shown next to the Check buttons. */
  lastCheck: null as { ok: boolean; text: string; time: string } | null,
})

export const settings = (): Settings | undefined => app.status?.settings

function isUiError(err: unknown): err is UiError {
  return typeof err === 'object' && err !== null && 'kind' in err
}

export const errorKind = (err: unknown): string => (isUiError(err) ? err.kind : 'other')

/** An actionable sentence for an error. The raw text is in the log. */
export function explain(err: unknown): string {
  switch (errorKind(err)) {
    case 'wrong-key':
      return t('That is not the right password or recovery key.')
    case 'session':
      return t('Your Google session expired. Sign in again to continue: nothing already sent is lost.')
    case 'corrupt':
    case 'missing-blob':
      return t('Part of this backup is damaged or missing. Run “Check backup” in Settings.')
    case 'read':
      return t('A folder could not be read. Check that it still exists.')
    case 'refused':
      return t('Google Drive refused the request. Your Drive may be full.')
    case 'network':
      return t('Could not reach Google Drive. Check your connection and try again.')
    case 'aborted':
      return t('Stopped.')
    case 'format':
      return t('This is not an XkyHold backup, or it was made by a newer version.')
    case 'exists':
      return t('There is already a backup there. Choose it to connect, or use another name or folder.')
    case 'not-empty':
      return t('This folder has other files in it. Choose an empty folder or a new one.')
    case 'busy':
      return t('Another computer is using this backup right now. Try again in a few minutes.')
    case 'read-only':
      return t('This backup was made by XkyHold in a browser. You can restore from it, but new backups need a backup made by XkyHold-wd.')
    case 'io':
    case 'not-found':
      return t('The backup could not be reached. If it is on a disk, check that the disk is connected.')
    case 'outside-home':
      return t('XkyHold-wd only works with what is inside your user folder.')
    case 'root':
      return t('XkyHold-wd does not run as administrator. Close it and open it normally, without “Run as administrator”.')
    case 'keyring':
      return t('The Windows Credential Manager could not be used. Try again; if it keeps failing, restart Windows.')
    case 'no-client':
      return t('This copy of XkyHold-wd was built without Google Drive. Use a folder on this computer or a disk.')
    case 'drive-missing':
      return t('This backup is no longer in your Google Drive. If you moved it to the trash, restore it from there and try again.')
    case 'working':
      return t('XkyHold-wd is busy with another task. Try again when it finishes.')
    case 'locked':
      return t('Open your backup first.')
    case 'running-here':
      return t('A backup is already running on this computer. Wait for it to finish.')
    case 'needs-key':
      return t('Automatic backups need this computer to keep the key of your backup. Tick “Open this backup on this computer without the password” first.')
    case 'no-scheduler':
      return t('Automatic backups are not available in this version yet. XkyHold-wd reminds you instead.')
    default:
      return t('Something went wrong. Please try again.')
  }
}

/** Shows an error in the banner and remembers whether signing in again helps. */
export function fail(err: unknown): void {
  app.error = explain(err)
  app.expired = errorKind(err) === 'session'
}

function applyTheme(theme: string): void {
  if (theme === 'light' || theme === 'dark') document.documentElement.dataset.theme = theme
  else delete document.documentElement.dataset.theme
}

export async function refresh(): Promise<Status> {
  const status = await api.status()
  app.status = status
  applyLanguage(status.settings.language)
  applyTheme(status.settings.theme)
  if (status.job && !app.job) {
    app.job = { kind: status.job as Job['kind'], files: 0, bytes: 0, current: '', cleaning: false }
  }
  return status
}

/** Opens the configured backup with the Credential Manager, or asks for the password. */
export async function openBackup(): Promise<void> {
  if (!app.status?.configured || app.status.open) return
  app.opening = true
  try {
    app.needsPassword = !(await api.openSaved())
    await refresh()
  } catch (err) {
    fail(err)
  } finally {
    app.opening = false
  }
}

export async function unlock(secret: Secret, remember: boolean): Promise<void> {
  await api.unlock(secret, remember)
  app.needsPassword = false
  app.error = null
  await refresh()
}

export async function signIn(): Promise<boolean> {
  app.signinUrl = null
  try {
    await api.signIn()
    app.error = null
    app.expired = false
    await refresh()
    if (app.status?.configured && !app.status.open) await openBackup()
    return true
  } catch (err) {
    // Cancelled by the user: nothing to explain.
    if (errorKind(err) !== 'aborted') fail(err)
    return false
  } finally {
    app.signinUrl = null
  }
}

/** Stops waiting for the browser to come back from Google. */
export const cancelSignIn = (): Promise<void> => api.cancelSignIn()

export async function saveSettings(patch: Parameters<typeof api.saveSettings>[0]): Promise<void> {
  try {
    const saved = await api.saveSettings(patch)
    if (app.status) app.status.settings = saved
    applyLanguage(saved.language)
    applyTheme(saved.theme)
  } catch (err) {
    fail(err)
  }
}

function startJob(kind: Job['kind']): void {
  app.job = { kind, files: 0, bytes: 0, current: '', cleaning: false }
  app.error = null
  app.notice = null
  app.noticeFolder = null
  app.noticeIssues = false
  app.noticeDetails = []
}

export async function backupNow(): Promise<void> {
  startJob('backup')
  try {
    await api.backupNow()
  } catch (err) {
    app.job = null
    fail(err)
  }
}

export async function restoreItems(
  snapshot: string,
  path: string[],
  ids: string[],
  original: boolean,
  folder: string,
  to: string | null,
  conflict: string,
): Promise<void> {
  startJob('restore')
  try {
    await api.restoreStart(snapshot, path, ids, original, folder, to, conflict)
  } catch (err) {
    app.job = null
    fail(err)
  }
}

export async function checkBackup(full: boolean): Promise<void> {
  startJob('check')
  try {
    await api.checkStart(full)
  } catch (err) {
    app.job = null
    fail(err)
  }
}

export const stopJob = (): Promise<void> => api.stopJob()

const num = (v: unknown): number => (typeof v === 'number' ? v : 0)

function finished(done: JobDone): void {
  app.job = null
  app.generation++
  if (app.closing) void api.closeNow()
  if (!done.ok) {
    fail(done.error)
    if (done.kind === 'check') app.lastCheck = { ok: false, text: app.error ?? '', time: new Date().toISOString() }
    void refresh()
    return
  }
  const s = done.summary ?? {}
  if (done.kind === 'backup') {
    const parts = [
      t('Backup complete: {files}, {size}; {sent} sent.', {
        files: fileCount(num(s.files)),
        size: formatBytes(num(s.bytes)),
        sent: formatBytes(num(s.uploaded)),
      }),
    ]
    if (num(s.skipped) > 0) parts.push(t('{n} could not be read.', { n: fileCount(num(s.skipped)) }))
    // Decision D2: the counts here, the list in a file on this computer.
    const held: string[] = []
    if (num(s.locked) > 0) held.push(t('{n} in use by another program', { n: num(s.locked) }))
    if (num(s.cloud_only) > 0) held.push(t('{n} online-only, not downloaded', { n: num(s.cloud_only) }))
    if (num(s.bad_names) > 0) held.push(t('{n} with a name Windows reports as invalid', { n: num(s.bad_names) }))
    if (held.length > 0) {
      parts.push(t('Not saved: {list}.', { list: held.join(', ') }))
      app.noticeIssues = typeof s.issues_file === 'string'
    }
    if (num(s.forgotten) === 1) parts.push(t('1 old backup removed.'))
    else if (num(s.forgotten) > 1) parts.push(t('{n} old backups removed.', { n: num(s.forgotten) }))
    if (s.cleanup_failed) parts.push(t('Old backups could not be removed this time; they will be next time.'))
    app.notice = parts.join(' ')
    app.reminderDismissed = false
  } else if (done.kind === 'restore') {
    const parts = [t('Restored {files} ({size}).', { files: fileCount(num(s.files)), size: formatBytes(num(s.bytes)) })]
    if (num(s.existing) > 0) parts.push(t('{n} already there were left as they were.', { n: num(s.existing) }))
    if (num(s.bad_names) + num(s.unknown) > 0)
      parts.push(t('{n} could not be restored here.', { n: num(s.bad_names) + num(s.unknown) }))
    // Decision D1: names Windows cannot take come back under another name.
    if (num(s.renamed) > 0) parts.push(t('{n} restored under another name, because Windows does not accept theirs.', { n: num(s.renamed) }))
    // Decision D3: links Windows refused, and how to allow them.
    if (num(s.links_refused) > 0)
      parts.push(
        t('{n} links were not restored: Windows only lets you create them with Developer Mode on (Settings › System › For developers).', {
          n: num(s.links_refused),
        }),
      )
    const list = (v: unknown): string[] => (Array.isArray(v) ? v.map(String) : [])
    app.noticeDetails = [...list(s.renamed_list), ...list(s.links_refused_list).map((p) => t('link not restored: {path}', { path: p }))]
    const more = num(s.renamed) + num(s.links_refused) - app.noticeDetails.length
    if (more > 0) app.noticeDetails.push(t('and {n} more, listed in the log', { n: more }))
    app.notice = parts.join(' ')
    app.noticeFolder = typeof s.folder === 'string' ? s.folder : null
  } else if (s.ok) {
    app.notice = t('Your backup is complete and intact: {n} pieces of data checked.', { n: num(s.blobs).toLocaleString() })
    app.lastCheck = { ok: true, text: app.notice, time: new Date().toISOString() }
  } else {
    app.error = t('Part of this backup is damaged or missing. Make a new backup now so that your files are saved again.')
    app.lastCheck = { ok: false, text: app.error, time: new Date().toISOString() }
  }
  void refresh()
}

export async function start(): Promise<void> {
  await on.progress((p) => {
    if (!app.job) return
    if (p.kind === 'cleanup') app.job.cleaning = true
    else Object.assign(app.job, { files: p.files, bytes: p.bytes, current: p.current })
  })
  await on.jobDone(finished)
  await on.signinUrl((url) => (app.signinUrl = url))
  await on.closeRequested(() => (app.closing = true))
  try {
    await refresh()
  } catch (err) {
    fail(err)
  }
  app.ready = true
  await openBackup()
  // Backups from the CLI (and, later, the scheduled task) run in another
  // process: notice them and their end.
  setInterval(() => {
    if (!app.job) void refresh().catch(() => {})
  }, 20_000)
}
