<script lang="ts">
  // Browse the folders of the user folder. In "leave-out" mode every folder
  // starts ticked and unticking leaves it out; in "only" mode ticking adds
  // it to the folders to save. Reads through the engine, which never
  // follows symbolic links.
  import { api } from '../lib/api'
  import type { FolderInfo } from '../lib/api'
  import { t } from '../lib/i18n.svelte'

  let {
    mode,
    chosen,
    toggle,
    close,
  }: {
    mode: 'leave-out' | 'only'
    /** Paths relative to home: left out, or chosen, depending on the mode. */
    chosen: string[]
    toggle: (path: string, on: boolean) => void
    close: () => void
  } = $props()

  let trail = $state<string[]>([])
  let items = $state<FolderInfo[] | null>(null)
  let failed = $state(false)

  const here = (name: string) => [...trail, name].join('/')
  const isChosen = (name: string) => chosen.includes(here(name))
  /** Shown ticked: kept (leave-out mode) or chosen (only mode). */
  const ticked = (f: FolderInfo) => (f.always_out ? false : mode === 'leave-out' ? !isChosen(f.name) : isChosen(f.name))

  $effect(() => {
    const path = trail.join('/')
    items = null
    failed = false
    let cancelled = false
    api.folders(path).then(
      (list) => {
        if (!cancelled) items = list
      },
      () => {
        if (!cancelled) failed = true
      },
    )
    return () => {
      cancelled = true
    }
  })

  function change(f: FolderInfo, on: boolean) {
    // leave-out mode: unticking adds to the list; only mode: ticking does.
    toggle(here(f.name), mode === 'leave-out' ? !on : on)
  }
</script>

<div class="tree stack">
  <p class="small muted">
    {mode === 'leave-out' ? t('Untick the folders you do not want in your backup.') : t('Tick the folders you want in your backup.')}
  </p>

  <nav class="crumbs small" aria-label={t('Folder')}>
    <button class="link" onclick={() => (trail = [])} disabled={trail.length === 0}>{t('User folder')}</button>
    {#each trail as name, i}
      <span aria-hidden="true">/</span>
      <button class="link" onclick={() => (trail = trail.slice(0, i + 1))} disabled={i === trail.length - 1}>{name}</button>
    {/each}
  </nav>

  {#if failed}
    <p class="small muted">{t('This folder could not be read.')}</p>
  {:else if items === null}
    <p class="small muted">{t('Loading…')}</p>
  {:else if items.length === 0}
    <p class="small muted">{t('No folders inside.')}</p>
  {:else}
    <ul class="list">
      {#each items as f (f.name)}
        <li>
          <label class="grow option">
            <input
              type="checkbox"
              checked={ticked(f)}
              disabled={f.always_out || !f.text_name}
              onchange={(e) => change(f, e.currentTarget.checked)}
            />
            <span class:out={!ticked(f)}>{f.name}</span>
            {#if f.always_out}
              <span class="small muted">{t('always left out')}</span>
            {:else if f.mount}
              <span class="small muted">{t('another disk')}</span>
            {:else if f.cloud}
              <span class="small muted">{t('kept in the cloud')}</span>
            {/if}
          </label>
          {#if !f.always_out && !f.mount && (mode === 'only' || ticked(f))}
            <button class="link" onclick={() => (trail = [...trail, f.name])} aria-label={t('Open {name}', { name: f.name })}>›</button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if mode === 'leave-out'}
    <p class="small muted">{t('Files already in earlier backups stay there until those backups are removed.')}</p>
  {/if}
  <div><button class="btn" onclick={close}>{t('Done')}</button></div>
</div>

<style>
  .tree {
    padding: 12px 14px;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
  }

  .crumbs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
  }

  .crumbs .link:disabled {
    color: var(--text);
    opacity: 1;
    font-weight: 600;
  }

  .option {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }

  .out {
    color: var(--muted);
    text-decoration: line-through;
  }
</style>
