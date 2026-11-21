<script lang="ts">
  import { formatBytes } from './lib/format'
  import { t } from './lib/i18n.svelte'
  import { reminder } from './lib/reminder'
  import { api } from './lib/api'
  import { app, backupNow, signIn, start, stopJob } from './state/app.svelte'
  import type { View } from './state/app.svelte'
  import Home from './ui/Home.svelte'
  import Restore from './ui/Restore.svelte'
  import Settings from './ui/Settings.svelte'
  import Wizard from './ui/Wizard.svelte'

  const tabs: { id: View; label: string }[] = [
    // i18n-keys
    { id: 'home', label: 'Home' },
    { id: 'restore', label: 'Restore' },
    { id: 'settings', label: 'Settings' },
  ]

  // The wizard stays up until it says it is done: the backup counts as set
  // up as soon as it is created, but the recovery key still has to be shown.
  let inWizard = $state(false)
  void start()
  $effect(() => {
    if (app.ready && app.status && !app.status.configured) inWizard = true
  })

  function finishWizard(view: View) {
    app.view = view
    inWizard = false
  }

  // A backup is never started for the user: when one is due they are told,
  // on whichever screen they are, and choose.
  const due = $derived(reminder(app.status?.settings.last_backup?.time, app.status?.settings.remind_days ?? 1))
  const remind = $derived(
    !!app.status?.open && app.status.writable && !app.job && due.due && !app.reminderDismissed && app.view !== 'home',
  )

  function backUpFromReminder() {
    app.view = 'home'
    void backupNow()
  }

  async function stopAndClose() {
    await stopJob()
    // The window closes when the job reports that it stopped.
  }
</script>

<div class="shell">
  <header class="topbar">
    <span class="brand"><img src="/icon.svg" alt="" width="28" height="28" />XkyHold-wd</span>
    {#if app.ready && !inWizard}
      <nav class="tabs" aria-label={t('Sections')}>
        {#each tabs as tab (tab.id)}
          <button class="tab" aria-current={app.view === tab.id ? 'page' : undefined} onclick={() => (app.view = tab.id)}>
            {t(tab.label)}
          </button>
        {/each}
      </nav>
    {/if}
  </header>

  <main class="page">
    {#if app.closing && app.job}
      <div class="banner warn" role="alert">
        <span>
          {app.job.kind === 'backup'
            ? t('A backup is running. If you close now it stops, and the next backup continues where it left off.')
            : t('XkyHold-wd is still working. Close anyway?')}
        </span>
        <button class="btn" onclick={() => (app.closing = false)}>{t('Keep working')}</button>
        <button class="btn danger" onclick={stopAndClose}>{t('Stop and close')}</button>
      </div>
    {/if}

    {#if !app.ready || !app.status}
      {#if app.error}
        <div class="banner warn" role="alert"><span>{app.error}</span></div>
      {:else}
        <p class="muted">{t('Loading…')}</p>
      {/if}
    {:else if inWizard}
      <Wizard done={finishWizard} />
    {:else}
      {#if app.error}
        <div class="banner warn" role="alert">
          <span>{app.error}</span>
          {#if app.expired}
            <button class="btn" onclick={signIn}>{t('Sign in again')}</button>
          {/if}
          <button class="link" onclick={() => (app.error = null)}>{t('Dismiss')}</button>
        </div>
      {/if}
      {#if app.notice}
        <div class="banner ok" role="status">
          <span>{app.notice}</span>
          {#if app.noticeFolder}
            <button class="btn" onclick={() => api.openFolder(app.noticeFolder!)}>{t('Open folder')}</button>
          {/if}
          {#if app.noticeIssues}
            <button class="btn" onclick={() => api.openIssues()}>{t('Show the list')}</button>
          {/if}
          <button class="link" onclick={() => (app.notice = null)}>{t('Dismiss')}</button>
          {#if app.noticeDetails.length > 0}
            <details class="details">
              <summary class="small">{t('Details')}</summary>
              <ul class="small">
                {#each app.noticeDetails as line, i (i)}<li>{line}</li>{/each}
              </ul>
            </details>
          {/if}
        </div>
      {/if}
      {#if app.signinUrl}
        <div class="banner info" role="status">
          <span>{t('If your browser did not open, copy this address into it:')} <span class="url">{app.signinUrl}</span></span>
        </div>
      {/if}

      {#if remind}
        <div class="banner info" role="status">
          <span>
            {#if due.days === null}
              {t('You have not made a backup yet.')}
            {:else if due.days <= 1}
              {t('Your last backup was more than a day ago.')}
            {:else}
              {t('Your last backup was {n} days ago.', { n: due.days })}
            {/if}
          </span>
          <button class="btn primary" onclick={backUpFromReminder}>{t('Back up now')}</button>
          <button class="btn" onclick={() => (app.reminderDismissed = true)}>{t('Not now')}</button>
        </div>
      {/if}

      {#if app.view === 'home'}
        <Home />
      {:else if app.view === 'restore'}
        <Restore />
      {:else}
        <Settings />
      {/if}
    {/if}
  </main>

  <footer class="footer">
    <span>XkyHold-wd v{app.status?.version ?? ''} <span class="credit">by xkyriver on xion</span></span>
    {#if app.job && app.view !== 'home' && app.job.kind === 'backup'}
      <span>{t('Backing up…')} {formatBytes(app.job.bytes)}</span>
    {/if}
  </footer>
</div>

<style>
  .details {
    flex-basis: 100%;
  }

  .details ul {
    margin: 4px 0 0;
    padding-left: 18px;
    max-height: 12em;
    overflow: auto;
    word-break: break-all;
  }

  .url {
    font-family: ui-monospace, monospace;
    word-break: break-all;
    user-select: all;
  }
</style>
