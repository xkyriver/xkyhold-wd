<script lang="ts">
  // First run (PLAN.md 4.7): where the backup goes (Google Drive or a
  // folder) -> what to save -> password -> recovery key -> reminder. An
  // existing backup found there shortens it to "password -> done".
  import { api } from '../lib/api'
  import type { FolderState, Target, What } from '../lib/api'
  import { t } from '../lib/i18n.svelte'
  import { REMINDER_CHOICES } from '../lib/reminder'
  import { app, cancelSignIn, explain, errorKind, refresh, saveSettings, signIn } from '../state/app.svelte'
  import type { View } from '../state/app.svelte'
  import What_ from './What.svelte'

  let { done }: { done: (view: View) => void } = $props()

  type Step = 'where' | 'google' | 'choose' | 'folder' | 'what' | 'password' | 'recovery' | 'remind' | 'unlock'
  const NEW = '\u0000new'
  const MIN_PASSWORD = 8

  const status = $derived(app.status!)
  let step = $state<Step>('where')
  let kind = $state<'drive' | 'local'>('drive')
  let busy = $state(false)
  let error = $state<string | null>(null)
  let expired = $state(false)
  let existing = $state<string[] | null>(null)
  let choice = $state(NEW)
  let name = $state('')
  let folder = $state('')
  let password = $state('')
  let password2 = $state('')
  let remember = $state(true)
  let useRecoveryKey = $state(false)
  let recoveryInput = $state('')
  let recoveryKey = $state('')
  let saved = $state(false)
  let copied = $state(false)
  let what = $state<What>({ only: [], leave_out: [], presets: [], patterns: [], cross_mounts: [] })
  let remindDays = $state(1)
  /** Connected to a backup that already existed (not creating one). */
  let connected = $state(false)
  let automatic = $state(false)

  $effect(() => {
    if (!name && status) name = defaultName(status.home)
  })

  /** The user's name, as in C:\Users\<name>, and the system. */
  function defaultName(home: string): string {
    return `${home.split('\\').filter(Boolean).at(-1) ?? 'user'}-windows`
  }

  const target = $derived<Target>(
    kind === 'drive' ? { kind: 'drive', name: choice === NEW ? name.trim() : choice } : { kind: 'local', path: folder.trim() },
  )
  const isNew = $derived(step !== 'unlock' && !connected)
  const steps = $derived<Step[]>(
    isNew
      ? ['where', kind === 'drive' ? 'choose' : 'folder', 'what', 'password', 'recovery', 'remind']
      : ['where', kind === 'drive' ? 'choose' : 'folder', 'unlock', 'what'],
  )
  const position = $derived(Math.max(0, steps.indexOf(step === 'google' ? 'choose' : step)))

  async function run(work: () => Promise<void>) {
    busy = true
    error = null
    expired = false
    try {
      await work()
    } catch (err) {
      error = explain(err)
      expired = errorKind(err) === 'session'
    } finally {
      busy = false
    }
  }

  function chooseWhere(k: 'drive' | 'local') {
    kind = k
    error = null
    if (k === 'local') step = 'folder'
    else if (status.settings.account) {
      step = 'choose'
      void loadBackups()
    } else step = 'google'
  }

  async function google() {
    busy = true
    const ok = await signIn()
    busy = false
    error = ok ? null : app.error
    app.error = null
    if (ok) {
      step = 'choose'
      void loadBackups()
    }
  }

  const loadBackups = () =>
    run(async () => {
      existing = await api.driveBackups()
      choice = existing.includes(name) ? name : NEW
    })

  function chosen() {
    error = null
    if (choice !== NEW) step = 'unlock'
    else if (existing?.includes(name.trim())) error = t('A backup with this name already exists. Choose another name.')
    else step = 'what'
  }

  async function pick() {
    const path = await api.pickFolder().catch(() => null)
    if (path) folder = path
  }

  const FOLDER_ERRORS: Partial<Record<FolderState, string>> = {
    // i18n-keys
    other: 'This folder has other files in it. Choose an empty folder or a new one.',
    'contains-home': 'Choose a folder outside your user folder, such as one on another disk.',
  }

  const checkFolder = () =>
    run(async () => {
      // A drive letter (E:\…) or a network share (\\server\share\…).
      if (!/^([a-zA-Z]:\\|\\\\[^\\]+\\[^\\]+)/.test(folder.trim())) {
        error = t('Write the full path of the folder, such as E:\\XkyHold or \\\\server\\share\\XkyHold.')
        return
      }
      const state = await api.inspectFolder(folder.trim())
      if (state === 'repository') step = 'unlock'
      else if (state === 'empty' || state === 'missing') step = 'what'
      else error = t(FOLDER_ERRORS[state]!)
    })

  const create = () =>
    run(async () => {
      await saveSettings({ what: $state.snapshot(what) })
      recoveryKey = await api.createBackup(target, password, remember && status.keyring)
      password = password2 = ''
      await refresh()
      step = 'recovery'
    })

  const unlock = () =>
    run(async () => {
      const writable = await api.connectBackup(
        target,
        useRecoveryKey ? { recovery_key: recoveryInput } : { password },
        remember && status.keyring,
      )
      password = ''
      await refresh()
      if (!writable) return done('restore')
      // A backup does not record "only some folders": confirm what to save
      // before the first backup from here, so nothing larger starts silently.
      what = $state.snapshot(app.status!.settings.what)
      connected = true
      step = 'what'
    })

  async function confirmWhat() {
    await saveSettings({ what: $state.snapshot(what) })
    done('home')
  }

  async function copy() {
    await navigator.clipboard.writeText(recoveryKey)
    copied = true
  }

  async function finish() {
    await saveSettings(automatic ? { automatic: 'daily' } : { remind_days: remindDays, automatic: 'off' })
    done('home')
  }

  const reminderLabel = (days: number) =>
    days === 0 ? t('Never') : days === 1 ? t('Every day') : days === 7 ? t('Every week') : t('Every {n} days', { n: days })
</script>

<div class="card stack">
  <ol class="dots" aria-label={t('Step {n} of {total}', { n: position + 1, total: steps.length })}>
    {#each steps as s, i (s)}<li class:on={i <= position}></li>{/each}
  </ol>

  {#if step === 'where'}
    <h1>{t('Keep your user folder safe')}</h1>
    <p class="muted">
      {t('XkyHold-wd saves your user folder, encrypted on this computer first, so only you can read it. Where should your backup go?')}
    </p>
    <div class="stack tight">
      <button class="choice" onclick={() => chooseWhere('drive')} disabled={!status.google_client}>
        <strong>{t('Google Drive')}</strong>
        <span class="small muted">
          {status.google_client
            ? t('In your own Drive. XkyHold-wd can only see the files it creates there.')
            : t('This copy of XkyHold-wd was built without Google Drive.')}
        </span>
      </button>
      <button class="choice" onclick={() => chooseWhere('local')}>
        <strong>{t('A folder or a disk')}</strong>
        <span class="small muted">{t('An external disk, a network share, or another folder on this computer.')}</span>
      </button>
    </div>
  {:else if step === 'google'}
    <h1>{t('Connect to Google Drive')}</h1>
    <p class="muted">{t('Your browser opens Google’s sign-in page. Come back here when you finish.')}</p>
    {#if !status.keyring}
      <p class="small muted">{t('The Windows Credential Manager cannot be used, so you will sign in to Google again each time you open XkyHold-wd.')}</p>
    {/if}
    <button class="btn primary big block" onclick={google} disabled={busy}>
      {busy ? t('Waiting for Google…') : t('Continue with Google')}
    </button>
    {#if busy && app.signinUrl}
      <p class="small muted">
        {t('If your browser did not open, copy this address into it:')}
        <span class="url">{app.signinUrl}</span>
      </p>
    {/if}
    <div class="row">
      {#if busy}
        <!-- The browser may never come back: the wrong one opened, or it was closed. -->
        <button class="btn" onclick={cancelSignIn}>{t('Cancel')}</button>
      {:else}
        <button class="btn" onclick={() => (step = 'where')}>{t('Back')}</button>
      {/if}
    </div>
  {:else if step === 'choose'}
    <h1>{t('Your backup')}</h1>
    {#if existing === null}
      <p class="muted">{error ? '' : t('Looking for your backups…')}</p>
      {#if error}<button class="btn" onclick={loadBackups} disabled={busy}>{t('Try again')}</button>{/if}
    {:else}
      {#if existing.length > 0}
        <p class="muted">{t('Connect to a backup you already have, or start a new one.')}</p>
        <div class="stack tight" role="radiogroup">
          {#each existing as backupName (backupName)}
            <label class="option"><input type="radio" bind:group={choice} value={backupName} /> {backupName}</label>
          {/each}
          <label class="option"><input type="radio" bind:group={choice} value={NEW} /> {t('Start a new backup')}</label>
        </div>
      {/if}
      {#if choice === NEW}
        <div>
          <label for="backup-name">{t('Name of this backup')}</label>
          <input id="backup-name" type="text" bind:value={name} autocomplete="off" spellcheck="false" />
          <p class="small muted">{t('Use a different name on each computer.')}</p>
        </div>
      {/if}
    {/if}
    <div class="row between">
      <button class="btn" onclick={() => (step = 'where')}>{t('Back')}</button>
      <button class="btn primary" onclick={chosen} disabled={existing === null || (choice === NEW && !name.trim())}>{t('Continue')}</button>
    </div>
  {:else if step === 'folder'}
    <h1>{t('Choose the folder')}</h1>
    <p class="muted">{t('Choose an empty folder for a new backup, or the folder of a backup you already have.')}</p>
    <div>
      <label for="folder">{t('Folder')}</label>
      <div class="row nowrap">
        <input id="folder" type="text" bind:value={folder} placeholder="E:\XkyHold" spellcheck="false" autocomplete="off" />
        <button class="btn" onclick={pick}>{t('Choose…')}</button>
      </div>
      <p class="small muted">{t('A new backup goes into this folder; it is created if it does not exist.')}</p>
    </div>
    <div class="row between">
      <button class="btn" onclick={() => (step = 'where')} disabled={busy}>{t('Back')}</button>
      <button class="btn primary" onclick={checkFolder} disabled={busy || !folder.trim()}>{t('Continue')}</button>
    </div>
  {:else if step === 'what' && connected}
    <h1>{t('What to save')}</h1>
    <p class="muted">{t('Check what this computer saves from now on. Earlier backups stay as they are.')}</p>
    <What_ {what} change={(w) => (what = w)} />
    <div class="row end">
      <button class="btn primary" onclick={confirmWhat}>{t('Finish')}</button>
    </div>
  {:else if step === 'what'}
    <h1>{t('What to save')}</h1>
    <What_ {what} change={(w) => (what = w)} />
    <div class="row between">
      <button class="btn" onclick={() => (step = kind === 'drive' ? 'choose' : 'folder')}>{t('Back')}</button>
      <button class="btn primary" onclick={() => (step = 'password')}>{t('Continue')}</button>
    </div>
  {:else if step === 'password'}
    <h1>{t('Protect your backup')}</h1>
    <p class="muted">{t('Your files are encrypted with this password before they leave this computer.')}</p>
    <div>
      <label for="pw">{t('Password')}</label>
      <input id="pw" type="password" bind:value={password} autocomplete="new-password" />
    </div>
    <div>
      <label for="pw2">{t('Confirm password')}</label>
      <input id="pw2" type="password" bind:value={password2} autocomplete="new-password" />
    </div>
    <p class="small muted">
      {#if password && password.length < MIN_PASSWORD}
        {t('Use at least {n} characters.', { n: MIN_PASSWORD })}
      {:else if password2 && password !== password2}
        {t('The two passwords are different.')}
      {:else}
        {t('Next you will get a recovery key, in case you forget this password.')}
      {/if}
    </p>
    {#if status.keyring}
      <label class="check"><input type="checkbox" bind:checked={remember} /> {t('Open this backup on this computer without the password')}</label>
    {/if}
    <div class="row between">
      <button class="btn" onclick={() => (step = 'what')} disabled={busy}>{t('Back')}</button>
      <button class="btn primary" onclick={create} disabled={busy || password.length < MIN_PASSWORD || password !== password2}>
        {busy ? t('Creating your backup…') : t('Create backup')}
      </button>
    </div>
  {:else if step === 'recovery'}
    <h1>{t('Save your recovery key')}</h1>
    <p class="muted">{t('If you forget your password, this key is the only other way to open your backup. It is shown only once.')}</p>
    <p class="key">{recoveryKey}</p>
    <div class="row">
      <button class="btn" onclick={() => window.print()}>{t('Print')}</button>
      <button class="btn" onclick={copy}>{copied ? t('Copied') : t('Copy')}</button>
    </div>
    <p class="small muted">{t('Keep it away from this computer: on paper, or in a password manager.')}</p>
    <label class="check"><input type="checkbox" bind:checked={saved} /> {t('I have saved my recovery key somewhere safe')}</label>
    <div class="row end">
      <button class="btn primary" onclick={() => (step = 'remind')} disabled={!saved}>{t('Continue')}</button>
    </div>
  {:else if step === 'remind' && !status.scheduler}
    <!-- Automatic backups come with the Task Scheduler (Phase 5). -->
    <h1>{t('Reminders')}</h1>
    <div>
      <label for="remind">{t('Remind me to back up')}</label>
      <select id="remind" bind:value={remindDays}>
        {#each REMINDER_CHOICES as days (days)}<option value={days}>{reminderLabel(days)}</option>{/each}
      </select>
    </div>
    <p class="small muted">{t('XkyHold-wd never starts a backup by itself unless you ask it to. It reminds you when one is due.')}</p>
    <p class="small muted">{t('You can change this later in Settings.')}</p>
    <div class="row end"><button class="btn primary" onclick={finish}>{t('Finish')}</button></div>
  {:else if step === 'remind'}
    <h1>{t('Reminders or automatic backups')}</h1>
    <div class="stack tight" role="radiogroup">
      <label class="option"><input type="radio" bind:group={automatic} value={false} /> {t('Remind me, and I start each backup')}</label>
      <label class="option" class:disabled={!remember || !status.keyring}>
        <input type="radio" bind:group={automatic} value={true} disabled={!remember || !status.keyring} />
        {t('Back up automatically every day')}
      </label>
    </div>
    {#if automatic}
      <p class="small muted">{t('XkyHold-wd backs up by itself, also with this window closed, and tells you only if something goes wrong.')}</p>
    {:else}
      <div>
        <label for="remind">{t('Remind me to back up')}</label>
        <select id="remind" bind:value={remindDays}>
          {#each REMINDER_CHOICES as days (days)}<option value={days}>{reminderLabel(days)}</option>{/each}
        </select>
      </div>
      {#if !status.keyring}
        <p class="small muted">{t('Automatic backups need the Windows Credential Manager, which cannot be used right now.')}</p>
      {:else if !remember}
        <p class="small muted">{t('Automatic backups need this computer to keep the key of your backup, which you chose not to.')}</p>
      {/if}
    {/if}
    <p class="small muted">{t('You can change this later in Settings.')}</p>
    <div class="row end"><button class="btn primary" onclick={finish}>{t('Finish')}</button></div>
  {:else if step === 'unlock'}
    <h1>{t('Open “{name}”', { name: kind === 'drive' ? choice : folder })}</h1>
    <p class="muted">{t('There is already a backup here. Open it with its password.')}</p>
    {#if useRecoveryKey}
      <div>
        <label for="rk">{t('Recovery key')}</label>
        <input id="rk" type="text" bind:value={recoveryInput} autocomplete="off" spellcheck="false" />
      </div>
    {:else}
      <div>
        <label for="pw">{t('Password')}</label>
        <input id="pw" type="password" bind:value={password} autocomplete="current-password" />
      </div>
    {/if}
    <button class="link small" onclick={() => (useRecoveryKey = !useRecoveryKey)}>
      {useRecoveryKey ? t('Use my password instead') : t('I forgot my password')}
    </button>
    {#if status.keyring}
      <label class="check"><input type="checkbox" bind:checked={remember} /> {t('Open this backup on this computer without the password')}</label>
    {/if}
    <div class="row between">
      <button class="btn" onclick={() => (step = kind === 'drive' ? 'choose' : 'folder')} disabled={busy}>{t('Back')}</button>
      <button class="btn primary" onclick={unlock} disabled={busy || !(useRecoveryKey ? recoveryInput.trim() : password)}>
        {busy ? t('Opening your backup…') : t('Open backup')}
      </button>
    </div>
  {/if}

  {#if error}
    <div class="banner warn" role="alert">
      <span>{error}</span>
      {#if expired}
        <button class="btn" onclick={google} disabled={busy}>{t('Sign in again')}</button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .dots {
    display: flex;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .dots li {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--border);
  }

  .dots li.on {
    background: var(--accent);
  }

  .choice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 14px 16px;
    font: inherit;
    color: inherit;
    text-align: left;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .choice:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .choice:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .option {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    padding: 10px 12px;
    font-weight: 500;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .option:has(input:checked) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .option.disabled {
    cursor: default;
    opacity: 0.6;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 400;
    cursor: pointer;
  }

  .nowrap {
    flex-wrap: nowrap;
  }

  .nowrap input {
    flex: 1;
  }

  .url {
    display: block;
    margin-top: 4px;
    font-family: ui-monospace, monospace;
    word-break: break-all;
    user-select: all;
  }

  .key {
    padding: 16px;
    font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
    font-size: 1rem;
    line-height: 1.9;
    text-align: center;
    word-spacing: 0.2em;
    background: var(--surface-2);
    border: 1px dashed var(--border);
    border-radius: var(--radius-sm);
    user-select: all;
  }
</style>
