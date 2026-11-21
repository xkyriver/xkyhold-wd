// The engine, as the window sees it: Tauri commands and events. Errors
// arrive as { kind, message }; `explain` in the app state turns the kind
// into a sentence.

import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'

export type Target = { kind: 'drive'; name: string } | { kind: 'local'; path: string }

export interface What {
  only: string[]
  leave_out: string[]
  presets: string[]
  patterns: string[]
  cross_mounts: string[]
}

export interface LastBackup {
  time: string
  files: number
  bytes: number
  skipped: number
  /** In use, online-only or with a bad name (decision D2). */
  not_stored: number
}

export interface Settings {
  account: string | null
  backup: Target | null
  repo_id: string | null
  what: What
  remind_days: number
  keep_months: number
  language: string | null
  theme: string
  last_backup: LastBackup | null
  automatic: 'off' | 'hourly' | 'daily' | 'weekly'
  notify_success: boolean
}

export interface Status {
  version: string
  home: string
  /** The computer's name. */
  device: string
  settings: Settings
  configured: boolean
  open: boolean
  writable: boolean
  remembered: boolean
  job: string | null
  running_here: boolean
  /** Automatic backups with the window closed (the Task Scheduler, Phase 5). */
  scheduler: boolean
  google_client: boolean
  /** The Credential Manager can be used. */
  keyring: boolean
  /** The list of what the last backup could not store exists. */
  issues_file: boolean
}

export interface SnapInfo {
  id: string
  time: string
  device: string
  files: number
  bytes: number
  skipped: number
  not_stored: number
  home: boolean
}

export interface EntryInfo {
  id: string
  name: string
  kind: 'file' | 'dir' | 'mount' | 'symlink' | 'unknown'
  size: number
  mtime: number | null
}

export interface FolderInfo {
  name: string
  text_name: boolean
  mount: boolean
  cloud: boolean
  always_out: boolean
}

export interface Secret {
  password?: string
  recovery_key?: string
}

export interface UiError {
  kind: string
  message: string
}

export interface ProgressEvent {
  kind: 'backup' | 'cleanup' | 'restore' | 'check'
  files: number
  bytes: number
  current: string
}

export interface JobDone {
  kind: 'backup' | 'restore' | 'check'
  ok: boolean
  error?: UiError
  summary?: Record<string, number | string | boolean | string[] | null>
}

export type FolderState = 'missing' | 'empty' | 'repository' | 'other' | 'contains-home'

export const api = {
  status: () => invoke<Status>('status'),
  openSaved: () => invoke<boolean>('open_saved'),
  signIn: () => invoke<string>('sign_in'),
  signOut: () => invoke<void>('sign_out'),
  cancelSignIn: () => invoke<void>('cancel_sign_in'),
  driveBackups: () => invoke<string[]>('drive_backups'),
  inspectFolder: (path: string) => invoke<FolderState>('inspect_folder', { path }),
  pickFolder: () => invoke<string | null>('pick_folder'),
  createBackup: (target: Target, password: string, remember: boolean) =>
    invoke<string>('create_backup', { target, password, remember }),
  connectBackup: (target: Target, secret: Secret, remember: boolean) =>
    invoke<boolean>('connect_backup', { target, secret, remember }),
  unlock: (secret: Secret, remember: boolean) => invoke<void>('unlock', { secret, remember }),
  backupNow: () => invoke<void>('backup_now'),
  stopJob: () => invoke<void>('stop_job'),
  snapshots: () => invoke<SnapInfo[]>('snapshots'),
  entries: (snapshot: string, path: string[]) => invoke<EntryInfo[]>('entries', { snapshot, path }),
  restoreExisting: (snapshot: string, path: string[], ids: string[]) =>
    invoke<string[]>('restore_existing', { snapshot, path, ids }),
  /** `to`: a folder the user chose, instead of %USERPROFILE%\\XkyHold-restore; null for the default. */
  restoreStart: (
    snapshot: string,
    path: string[],
    ids: string[],
    original: boolean,
    folder: string,
    to: string | null,
    conflict: string,
  ) => invoke<void>('restore_start', { snapshot, path, ids, original, folder, to, conflict }),
  checkStart: (full: boolean) => invoke<void>('check_start', { full }),
  saveSettings: (
    patch: Partial<
      Pick<Settings, 'what' | 'remind_days' | 'keep_months' | 'language' | 'theme' | 'automatic' | 'notify_success'>
    >,
  ) =>
    invoke<Settings>('save_settings', { patch }),
  changePassword: (secret: Secret, newPassword: string) => invoke<void>('change_password', { secret, newPassword }),
  setRemember: (remember: boolean) => invoke<void>('set_remember', { remember }),
  forgetComputer: () => invoke<void>('forget_computer'),
  folders: (path: string) => invoke<FolderInfo[]>('folders', { path }),
  presets: () => invoke<string[]>('presets'),
  countSizes: () => invoke<void>('count_sizes'),
  openFolder: (path: string) => invoke<void>('open_folder', { path }),
  openIssues: () => invoke<void>('open_issues'),
  closeNow: () => invoke<void>('close_now'),
}

export const on = {
  progress: (f: (p: ProgressEvent) => void): Promise<UnlistenFn> => listen<ProgressEvent>('progress', (e) => f(e.payload)),
  jobDone: (f: (d: JobDone) => void): Promise<UnlistenFn> => listen<JobDone>('job-done', (e) => f(e.payload)),
  sizes: (f: (s: Record<string, number>) => void): Promise<UnlistenFn> =>
    listen<Record<string, number>>('sizes', (e) => f(e.payload)),
  signinUrl: (f: (url: string) => void): Promise<UnlistenFn> => listen<string>('signin-url', (e) => f(e.payload)),
  closeRequested: (f: () => void): Promise<UnlistenFn> => listen('close-requested', () => f()),
}
