<script lang="ts">
  import { api } from '../lib/api'
  import type { Settings } from '../lib/api'
  import { formatWhen, LANGUAGES, t } from '../lib/i18n.svelte'
  import { KEEP_CHOICES, REMINDER_CHOICES } from '../lib/reminder'
  import { app, checkBackup, explain, refresh, saveSettings, signIn, stopJob } from '../state/app.svelte'
  import What from './What.svelte'

  const MIN_PASSWORD = 8

  let changing = $state(false)
  let useRecoveryKey = $state(false)
  let current = $state('')
  let next = $state('')
  let next2 = $state('')
  let busy = $state(false)
  let message = $state<{ ok: boolean; text: string } | null>(null)
  let confirmForget = $state(false)

  const status = $derived(app.status!)
  const s = $derived(status.settings)
  const target = $derived(s.backup)

  const reminderLabel = (days: number) =>
    days === 0 ? t('Never') : days === 1 ? t('Every day') : days === 7 ? t('Every week') : t('Every {n} days', { n: days })
  const AUTOMATIC: { id: Settings['automatic']; label: string }[] = [
    // i18n-keys
    { id: 'off', label: 'Off' },
    { id: 'hourly', label: 'Every hour' },
    { id: 'daily', label: 'Every day' },
    { id: 'weekly', label: 'Every week' },
  ]
  const keepLabel = (months: number) =>
    months === 0 ? t('Forever') : months === 12 ? t('At least a year') : t('At least {n} months', { n: months })

  async function savePassword() {
    busy = true
    message = null
    try {
      await api.changePassword(useRecoveryKey ? { recovery_key: current } : { password: current }, next)
      message = { ok: true, text: t('Your password was changed.') }
      changing = false
      current = next = next2 = ''
    } catch (err) {
      message = { ok: false, text: explain(err) }
    } finally {
      busy = false
    }
  }

  async function remember(on: boolean) {
    try {
      await api.setRemember(on)
      await refresh()
    } catch (err) {
      message = { ok: false, text: explain(err) }
    }
  }

  async function signOut() {
    await api.signOut().catch((err) => (message = { ok: false, text: explain(err) }))
    await refresh()
  }

  async function forget() {
    await api.forgetComputer().catch((err) => (message = { ok: false, text: explain(err) }))
    await refresh()
  }
</script>

<section class="card stack">
  <h2>{t('What to save')}</h2>
  <What what={s.what} change={(what) => saveSettings({ what })} />
</section>

<section class="card stack">
  <h2>{t('Backups')}</h2>
  <!-- Automatic backups need the Task Scheduler (Phase 5); until then the window only reminds. -->
  {#if status.scheduler}
  <div>
    <label for="automatic">{t('Back up automatically')}</label>
    <select
      id="automatic"
      value={s.automatic}
      disabled={s.automatic === 'off' && !status.remembered}
      onchange={(e) => saveSettings({ automatic: e.currentTarget.value as Settings['automatic'] })}
    >
      {#each AUTOMATIC as choice (choice.id)}<option value={choice.id}>{t(choice.label)}</option>{/each}
    </select>
    <p class="small muted">
      {#if s.automatic !== 'off'}
        {t('XkyHold-wd backs up by itself, also with this window closed, and tells you only if something goes wrong.')}
      {:else if !status.remembered}
        {t('To back up automatically, first tick “Open this backup on this computer without the password” under Password.')}
      {:else}
        {t('Off: XkyHold-wd only reminds you. Backups start when you choose.')}
      {/if}
    </p>
  </div>
  {/if}
  {#if status.scheduler && s.automatic !== 'off'}
    <label class="check">
      <input type="checkbox" checked={s.notify_success} onchange={(e) => saveSettings({ notify_success: e.currentTarget.checked })} />
      {t('Also tell me when an automatic backup goes well')}
    </label>
  {:else}
    <div>
      <label for="remind">{t('Remind me to back up')}</label>
      <select id="remind" value={s.remind_days} onchange={(e) => saveSettings({ remind_days: Number(e.currentTarget.value) })}>
        {#each REMINDER_CHOICES as days (days)}<option value={days}>{reminderLabel(days)}</option>{/each}
      </select>
      <p class="small muted">
        {status.scheduler
          ? t('Reminders appear as notifications, also with this window closed.')
          : t('Reminders appear in this window when you open it.')}
      </p>
    </div>
  {/if}
  {#if status.scheduler && !status.task && (s.automatic !== 'off' || s.remind_days > 0)}
    <div class="banner warn">
      <span>{t('Windows did not let XkyHold-wd create its scheduled task, so reminders and automatic backups only happen while this window is open.')}</span>
    </div>
  {/if}
  <div>
    <label for="keep">{t('Keep backups')}</label>
    <select id="keep" value={s.keep_months} onchange={(e) => saveSettings({ keep_months: Number(e.currentTarget.value) })}>
      {#each KEEP_CHOICES as months (months)}<option value={months}>{keepLabel(months)}</option>{/each}
    </select>
    <p class="small muted">
      {target?.kind === 'drive'
        ? t('Older backups are removed after each new backup. They stay in the Google Drive trash for 30 days.')
        : t('Older backups are removed after each new backup. They stay in the backup’s trash folder for 30 days.')}
    </p>
  </div>
</section>

<section class="card stack">
  <h2>{t('Check backup')}</h2>
  <p class="small muted">{t('Makes sure every part of your backup is there. The full check also reads all of it back, which takes longer.')}</p>
  <div class="row">
    <button class="btn" onclick={() => checkBackup(false)} disabled={!status.open || !!app.job}>{t('Check')}</button>
    <button class="btn" onclick={() => checkBackup(true)} disabled={!status.open || !!app.job}>{t('Full check')}</button>
    {#if app.job?.kind === 'check'}
      <span class="small muted">
        {t('Checking…')}
        {#if app.job.files > 0}{t('{n} pieces of data checked', { n: app.job.files.toLocaleString() })}{/if}
      </span>
      <button class="link small" onclick={stopJob}>{t('Stop')}</button>
    {/if}
  </div>
  {#if app.lastCheck && app.job?.kind !== 'check'}
    <div class="banner {app.lastCheck.ok ? 'ok' : 'warn'}" role="status">
      <span>{formatWhen(app.lastCheck.time)}: {app.lastCheck.text}</span>
    </div>
  {/if}
</section>

<section class="card stack">
  <h2>{t('Where your backup is')}</h2>
  {#if target?.kind === 'drive'}
    <p class="muted small">
      {t('Google Drive · {name}', { name: target.name })}{#if s.account}&nbsp;· {s.account}{/if}
    </p>
    <div class="row">
      {#if s.account}
        <button class="btn" onclick={signOut} disabled={!!app.job}>{t('Sign out of Google')}</button>
      {:else}
        <button class="btn primary" onclick={signIn}>{t('Continue with Google')}</button>
      {/if}
    </div>
  {:else if target}
    <p class="muted small">{target.path}</p>
  {/if}
</section>

<section class="card stack">
  <h2>{t('Password')}</h2>
  {#if status.keyring && status.open}
    <label class="check">
      <input type="checkbox" checked={status.remembered} onchange={(e) => remember(e.currentTarget.checked)} />
      {t('Open this backup on this computer without the password')}
    </label>
  {/if}
  {#if !changing}
    <div><button class="btn" onclick={() => ((changing = true), (message = null))} disabled={!status.open}>{t('Change password…')}</button></div>
  {:else}
    <div>
      <label for="cur">{useRecoveryKey ? t('Recovery key') : t('Current password')}</label>
      <input id="cur" type={useRecoveryKey ? 'text' : 'password'} bind:value={current} autocomplete="off" spellcheck="false" />
      <button class="link small" onclick={() => ((useRecoveryKey = !useRecoveryKey), (current = ''))}>
        {useRecoveryKey ? t('Use my password instead') : t('I forgot my password')}
      </button>
    </div>
    <div>
      <label for="new">{t('New password')}</label>
      <input id="new" type="password" bind:value={next} autocomplete="new-password" />
    </div>
    <div>
      <label for="new2">{t('Confirm password')}</label>
      <input id="new2" type="password" bind:value={next2} autocomplete="new-password" />
    </div>
    {#if next && next.length < MIN_PASSWORD}
      <p class="small muted">{t('Use at least {n} characters.', { n: MIN_PASSWORD })}</p>
    {:else if next2 && next !== next2}
      <p class="small muted">{t('The two passwords are different.')}</p>
    {/if}
    <div class="row end">
      <button class="btn" onclick={() => (changing = false)} disabled={busy}>{t('Cancel')}</button>
      <button class="btn primary" onclick={savePassword} disabled={busy || !current || next.length < MIN_PASSWORD || next !== next2}>
        {t('Change password')}
      </button>
    </div>
  {/if}
  {#if message}
    <div class="banner {message.ok ? 'ok' : 'warn'}" role="status"><span>{message.text}</span></div>
  {/if}
</section>

<section class="card stack">
  <h2>{t('Appearance')}</h2>
  <div class="row">
    <div class="grow">
      <label for="lang">{t('Language')}</label>
      <select id="lang" value={s.language ?? ''} onchange={(e) => saveSettings({ language: e.currentTarget.value })}>
        <option value="">{t('Same as the system')}</option>
        {#each LANGUAGES as language (language.id)}<option value={language.id}>{language.name}</option>{/each}
      </select>
    </div>
    <div class="grow">
      <label for="theme">{t('Theme')}</label>
      <select id="theme" value={s.theme} onchange={(e) => saveSettings({ theme: e.currentTarget.value })}>
        <option value="system">{t('Same as the system')}</option>
        <option value="light">{t('Light')}</option>
        <option value="dark">{t('Dark')}</option>
      </select>
    </div>
  </div>
</section>

<section class="card stack">
  <h2>{t('About')}</h2>
  <p>XkyHold-wd {status.version}</p>
  <p class="muted small">{t('XkyHold-wd does not look for new versions by itself. New versions are announced on the XkyHold site.')}</p>
</section>

<section class="card stack">
  <h2>{t('This computer')}</h2>
  <p class="muted small">
    {t('Removes XkyHold-wd’s settings, the key of your backup and the Google sign-in from this computer. Your backup stays where it is, and you can connect to it again with your password.')}
  </p>
  {#if !confirmForget}
    <div><button class="btn danger" onclick={() => (confirmForget = true)} disabled={!!app.job}>{t('Remove from this computer…')}</button></div>
  {:else}
    <div class="row">
      <button class="btn danger" onclick={forget}>{t('Yes, remove')}</button>
      <button class="btn" onclick={() => (confirmForget = false)}>{t('Cancel')}</button>
    </div>
  {/if}
</section>

<style>
  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 400;
    cursor: pointer;
  }

  .grow {
    flex: 1;
    min-width: 160px;
  }
</style>
