<script lang="ts">
  // Pick a backup by date, browse it, tick what to bring back, then choose
  // where: a new folder in the user folder (the default), a new folder inside a folder
  // the user chooses (a disk), or where it was. In the original
  // place nothing is replaced silently: what already exists is shown and
  // the user chooses to skip it, replace it or keep both.
  import { api } from '../lib/api'
  import type { EntryInfo, SnapInfo } from '../lib/api'
  import { formatBytes } from '../lib/format'
  import { fileCount, formatWhen, t } from '../lib/i18n.svelte'
  import { app, explain, restoreItems, stopJob } from '../state/app.svelte'
  import Unlock from './Unlock.svelte'

  interface Level {
    name: string
    /** Names in base64, from the backup's top. */
    path: string[]
    entries: EntryInfo[]
  }

  let list = $state.raw<SnapInfo[] | null>(null)
  let snapshot = $state.raw<SnapInfo | null>(null)
  let levels = $state.raw<Level[]>([])
  let ticked = $state<Record<string, boolean>>({})
  let loading = $state(false)
  let error = $state<string | null>(null)
  /** Where to: a new folder in home, a folder the user chose, or where it was. */
  let where = $state<'home' | 'other' | 'original'>('home')
  const original = $derived(where === 'original')
  let otherFolder = $state<string | null>(null)
  let existing = $state<string[] | null>(null)
  let conflict = $state('keep-both')

  const status = $derived(app.status!)
  const level = $derived(levels.at(-1))
  const chosen = $derived(level ? level.entries.filter((e) => ticked[e.id]) : [])
  const working = $derived(app.job?.kind === 'restore' || app.job?.kind === 'check')

  // Reload after each job, not on every status refresh: listing a Drive
  // backup takes a moment and holds it.
  const isOpen = $derived(status.open)
  const idle = $derived(!app.job)
  $effect(() => {
    void app.generation
    if (isOpen && idle) {
      api.snapshots().then(
        (s) => (list = s),
        (err) => (error = explain(err)),
      )
    }
  })

  async function load(name: string, path: string[]) {
    if (!snapshot || loading) return
    loading = true
    error = null
    try {
      const entries = await api.entries(snapshot.id, path)
      levels = [...levels, { name, path, entries }]
      ticked = {}
      existing = null
    } catch (err) {
      error = explain(err)
    } finally {
      loading = false
    }
  }

  async function open(s: SnapInfo) {
    snapshot = s
    levels = []
    where = 'home'
    await load(formatWhen(s.time), [])
  }

  function goTo(index: number) {
    levels = levels.slice(0, index + 1)
    ticked = {}
    existing = null
  }

  function back() {
    if (levels.length > 1) goTo(levels.length - 2)
    else snapshot = null
  }

  const toggleAll = (on: boolean) => (ticked = on && level ? Object.fromEntries(level.entries.map((e) => [e.id, true])) : {})

  // Going back to the original place: first see what is already there.
  $effect(() => {
    const ids = chosen.map((e) => e.id)
    if (!original || !snapshot || !level || ids.length === 0) {
      existing = null
      return
    }
    let cancelled = false
    api.restoreExisting(snapshot.id, level.path, ids).then(
      (names) => {
        if (!cancelled) existing = names
      },
      (err) => (error = explain(err)),
    )
    return () => {
      cancelled = true
    }
  })

  /** The system's folder chooser; going back to the default if nothing is chosen. */
  async function pickOther() {
    const picked = await api.pickFolder().catch(() => null)
    if (picked) otherFolder = picked
    else if (!otherFolder) where = 'home'
  }

  function restore() {
    if (!snapshot || !level) return
    void restoreItems(
      snapshot.id,
      level.path,
      chosen.map((e) => e.id),
      original,
      folderName,
      where === 'other' ? otherFolder : null,
      original ? conflict : 'keep-both',
    )
  }

  const icon = (e: EntryInfo) => (e.kind === 'dir' ? '📁' : e.kind === 'symlink' ? '↪' : e.kind === 'mount' ? '💽' : '')
  /** "2026-11-14 10.30": the backup's date and time here, safe as a folder name. */
  const folderName = $derived.by(() => {
    if (!snapshot) return ''
    const d = new Date(snapshot.time)
    const two = (n: number) => String(n).padStart(2, '0')
    return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())} ${two(d.getHours())}.${two(d.getMinutes())}`
  })
  const newFolder = $derived(`${status.home}\\XkyHold-restore\\${folderName}\\`)
</script>

{#if working && app.job}
  <section class="card center stack">
    <h1>{app.job.kind === 'check' ? t('Checking your backup…') : t('Restoring…')}</h1>
    {#if app.job.kind === 'restore'}
      <p class="muted">{fileCount(app.job.files)} · {formatBytes(app.job.bytes)}</p>
      <p class="small muted current">{app.job.current || ' '}</p>
    {:else}
      <p class="muted">{t('{n} pieces of data checked', { n: app.job.files.toLocaleString() })}</p>
    {/if}
    <div><button class="btn" onclick={stopJob}>{t('Stop')}</button></div>
  </section>
{:else if !status.open}
  {#if app.needsPassword}
    <Unlock />
  {:else}
    <section class="card stack">
      <h1>{t('Restore')}</h1>
      <p class="muted">{app.opening ? t('Opening your backup…') : t('Open your backup first.')}</p>
    </section>
  {/if}
{:else if !snapshot}
  <section class="card stack">
    <h1>{t('Restore')}</h1>
    {#if list === null}
      <p class="muted">{error ?? t('Loading…')}</p>
    {:else if list.length === 0}
      <p class="muted">{t('There is nothing to restore yet.')}</p>
    {:else}
      <p class="muted">{t('Pick a date to see what is inside.')}</p>
      <ul class="list">
        {#each list as s (s.id)}
          <li>
            <button class="pick grow" onclick={() => open(s)}>
              <strong>{formatWhen(s.time)}</strong>
              <span class="small muted">{fileCount(s.files)} · {formatBytes(s.bytes)} · {s.device}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{:else if level}
  <section class="card stack">
    <nav class="crumbs small" aria-label={t('Location')}>
      {#each levels as crumb, i (i)}
        {#if i > 0}<span class="muted">›</span>{/if}
        {#if i < levels.length - 1}
          <button class="link" onclick={() => goTo(i)}>{crumb.name}</button>
        {:else}
          <strong>{crumb.name}</strong>
        {/if}
      {/each}
    </nav>

    {#if level.entries.length === 0}
      <p class="muted">{t('This folder is empty.')}</p>
    {:else}
      <div class="row between small">
        <span class="muted">{t('Tick the files and folders you want back.')}</span>
        <button class="link" onclick={() => toggleAll(chosen.length < level.entries.length)}>
          {chosen.length < level.entries.length ? t('Select all') : t('Select none')}
        </button>
      </div>
      <ul class="list">
        {#each level.entries as entry (entry.id)}
          <li>
            <input type="checkbox" bind:checked={ticked[entry.id]} aria-label={entry.name} disabled={entry.kind === 'mount'} />
            {#if entry.kind === 'dir'}
              <button class="pick grow" onclick={() => load(entry.name, [...level.path, entry.id])} disabled={loading}>
                <span>{icon(entry)} {entry.name}</span>
              </button>
              <span class="muted" aria-hidden="true">›</span>
            {:else}
              <span class="grow">{icon(entry)} {entry.name}</span>
              {#if entry.kind === 'mount'}
                <span class="small muted">{t('another disk, not in the backup')}</span>
              {:else if entry.kind === 'file'}
                <span class="small muted">{formatBytes(entry.size)}</span>
              {/if}
            {/if}
          </li>
        {/each}
      </ul>
    {/if}

    {#if chosen.length > 0}
      <fieldset class="stack tight">
        <legend>{t('Restore to')}</legend>
        <label class="check"><input type="radio" bind:group={where} value="home" /> {t('A new folder: {path}', { path: newFolder })}</label>
        <label class="check">
          <input type="radio" bind:group={where} value="other" onchange={pickOther} />
          {otherFolder ? t('A new folder: {path}', { path: `${otherFolder.replace(/\\$/, '')}\\${folderName}\\` }) : t('Another folder, such as a disk…')}
        </label>
        {#if where === 'other' && otherFolder}
          <div><button class="link small" onclick={pickOther}>{t('Choose another folder…')}</button></div>
        {/if}
        {#if snapshot.home}
          <label class="check"><input type="radio" bind:group={where} value="original" /> {t('Where they were, in your user folder')}</label>
        {/if}
      </fieldset>

      {#if original && existing && existing.length > 0}
        <div class="banner warn stack tight">
          <span>
            {t('Already in your user folder: {list}', { list: existing.slice(0, 5).join(', ') + (existing.length > 5 ? '…' : '') })}
          </span>
          <div class="stack tight">
            <label class="check"><input type="radio" bind:group={conflict} value="keep-both" /> {t('Keep both: put the restored copy next to it')}</label>
            <label class="check"><input type="radio" bind:group={conflict} value="skip" /> {t('Skip: leave what is there')}</label>
            <label class="check"><input type="radio" bind:group={conflict} value="replace" /> {t('Replace it with the backup’s version')}</label>
          </div>
        </div>
      {/if}
    {/if}

    {#if error}<div class="banner warn" role="alert"><span>{error}</span></div>{/if}

    <div class="row between">
      <button class="btn" onclick={back}>{t('Back')}</button>
      <button
        class="btn primary"
        onclick={restore}
        disabled={chosen.length === 0 || !!app.job || (original && existing === null) || (where === 'other' && !otherFolder)}
      >
        {chosen.length === 0 ? t('Restore') : t('Restore {n}', { n: chosen.length })}
      </button>
    </div>
    <p class="small muted">{t('Restored files keep their dates and their read-only and hidden marks.')}</p>
  </section>
{/if}

<style>
  .pick {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 2px 0;
    font: inherit;
    color: inherit;
    text-align: left;
    background: none;
    border: 0;
    cursor: pointer;
  }

  .pick:hover strong,
  .pick:hover span:first-child {
    color: var(--accent);
  }

  .crumbs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .current {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  input[type='checkbox'] {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-bottom: 6px;
    font-weight: 600;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    font-weight: 400;
    cursor: pointer;
  }

  .banner.stack {
    flex-direction: column;
    align-items: stretch;
  }
</style>
