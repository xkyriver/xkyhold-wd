// When to remind the user to back up. Like XkyHold, the window never starts
// a backup by itself: it says that one is due and lets the user decide.
// Automatic backups (a systemd timer) are a separate, opt-in setting.

/** Days between reminders offered in Settings. 0 turns reminders off. */
export const REMINDER_CHOICES = [1, 2, 3, 7, 0] as const

/** Months of history offered in Settings. 0 keeps everything. */
export const KEEP_CHOICES = [0, 3, 6, 12] as const

const DAY_MS = 24 * 60 * 60_000

export interface Reminder {
  due: boolean
  /** Whole days since the last backup; null when there is none yet. */
  days: number | null
}

export function reminder(lastBackup: string | null | undefined, everyDays: number, now: Date = new Date()): Reminder {
  if (!lastBackup) return { due: everyDays > 0, days: null }
  const elapsed = now.getTime() - Date.parse(lastBackup)
  return { due: everyDays > 0 && elapsed >= everyDays * DAY_MS, days: Math.max(0, Math.floor(elapsed / DAY_MS)) }
}
