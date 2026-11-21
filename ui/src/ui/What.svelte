<script lang="ts">
  // What goes into the backup (PLAN.md 4.6): the whole user folder by
  // default, or chosen folders; folders, suggestions and patterns to leave
  // out. Links, junctions and mount points are never followed.
  import { onMount } from 'svelte'
  import { api, on } from '../lib/api'
  import type { FolderInfo, What } from '../lib/api'
  import { formatBytes } from '../lib/format'
  import { t } from '../lib/i18n.svelte'
  import Tree from './Tree.svelte'

  let { what, change }: { what: What; change: (what: What) => void } = $props()

  const PRESET_LABELS: Record<string, string> = {
    // i18n-keys
    'node-modules': 'Node.js packages (node_modules)',
    'rust-target': 'Rust build folders (target, next to Cargo.toml)',
    'maven-target': 'Maven build folders (target, next to pom.xml)',
    'python-venv': 'Python environments (.venv)',
    'browser-caches': 'Browser caches (Chrome, Edge, Firefox)',
    'app-caches': 'App caches (AppData)',
    containers: 'Docker data',
    'wsl-disks': 'WSL disks (ext4.vhdx)',
    steam: 'Steam libraries',
    'vm-images': 'Virtual machine disks (.vhdx, .vdi, .vmdk)',
    downloads: 'The Downloads folder',
  }

  let browsing = $state(false)
  let sizes = $state<Record<string, number> | null>(null)
  let top = $state<FolderInfo[]>([])
  /** The suggestions this build of the engine knows, in its order. */
  let known = $state<string[]>([])
  let patternText = $state('')
  // Follows the saved patterns; edits are saved when the field loses focus.
  $effect.pre(() => {
    patternText = what.patterns.join('\n')
  })

  let choosingOnly = $state(false)
  const only = $derived(what.only.length > 0 || choosingOnly)
  // Only matters when the whole user folder is saved.
  const cloud = $derived(only ? [] : top.filter((f) => f.cloud && !what.leave_out.includes(f.name)))

  onMount(() => {
    let stop: (() => void) | undefined
    void on.sizes((s) => (sizes = s)).then((u) => (stop = u))
    void api.countSizes()
    void api.folders('').then((list) => (top = list), () => {})
    void api.presets().then((list) => (known = list.filter((id) => id in PRESET_LABELS)), () => {})
    return () => stop?.()
  })

  const update = (patch: Partial<What>) => change({ ...what, ...patch })
  /** Paths as Windows shows them. */
  const shown = (paths: string[]) => paths.map((p) => p.replaceAll('/', '\\')).join(', ')

  function toggleIn(list: string[], path: string, on: boolean): string[] {
    const rest = list.filter((p) => p !== path && !p.startsWith(path + '/'))
    return on ? [...rest, path].sort() : rest
  }

  function setMode(toOnly: boolean) {
    choosingOnly = toOnly
    browsing = toOnly
    if (!toOnly) update({ only: [] })
  }

  function savePatterns() {
    const patterns = patternText
      .split('\n')
      .map((p) => p.trim())
      .filter(Boolean)
    if (patterns.join('\n') !== what.patterns.join('\n')) update({ patterns })
  }

  const presetSize = (id: string) => (sizes ? formatBytes(sizes[id] ?? 0) : t('counting…'))
</script>

<div class="stack">
  <div class="stack tight" role="radiogroup" aria-label={t('What to save')}>
    <label class="option"><input type="radio" checked={!only} onchange={() => setMode(false)} /> {t('My whole user folder')}</label>
    <label class="option"><input type="radio" checked={only} onchange={() => setMode(true)} /> {t('Only some folders')}</label>
  </div>

  {#if only}
    <p class="small muted">
      {what.only.length === 0 ? t('No folders chosen yet.') : t('Saving: {list}', { list: shown(what.only) })}
    </p>
  {:else}
    <p class="small muted">
      {what.leave_out.length === 0 ? t('Nothing left out.') : t('Left out: {list}', { list: shown(what.leave_out) })}
    </p>
  {/if}

  {#if browsing}
    {#if only}
      <Tree mode="only" chosen={what.only} toggle={(p, on) => update({ only: toggleIn(what.only, p, on) })} close={() => (browsing = false)} />
    {:else}
      <Tree
        mode="leave-out"
        chosen={what.leave_out}
        toggle={(p, on) => update({ leave_out: toggleIn(what.leave_out, p, on) })}
        close={() => (browsing = false)}
      />
    {/if}
  {:else}
    <div><button class="btn" onclick={() => (browsing = true)}>{only ? t('Choose folders…') : t('Leave out…')}</button></div>
  {/if}

  {#each cloud as f (f.name)}
    <div class="banner info">
      <span>{t('{name} is already kept in the cloud. You may not need it in this backup too.', { name: f.name })}</span>
      <button class="btn" onclick={() => update({ leave_out: toggleIn(what.leave_out, f.name, true) })}>{t('Leave it out')}</button>
    </div>
  {/each}

  <fieldset class="stack tight">
    <legend>{t('Suggestions to leave out')}</legend>
    {#each known as id (id)}
      <label class="check">
        <input
          type="checkbox"
          checked={what.presets.includes(id)}
          onchange={(e) => update({ presets: toggleIn(what.presets, id, e.currentTarget.checked) })}
        />
        <span class="grow">{t(PRESET_LABELS[id])}</span>
        <span class="small muted">{presetSize(id)}</span>
      </label>
    {/each}
  </fieldset>

  <details>
    <summary class="small">{t('Leave out by pattern')}</summary>
    <div class="stack tight">
      <label for="patterns" class="small muted">{t('One pattern per line, for example **/*.iso')}</label>
      <textarea id="patterns" rows="3" bind:value={patternText} onblur={savePatterns} spellcheck="false"></textarea>
    </div>
  </details>

  <p class="small muted">{t('Always left out: temporary files (AppData\\Local\\Temp) and the Windows registry files. Online-only OneDrive files are counted but never downloaded.')}</p>
</div>

<style>
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

  .grow {
    flex: 1;
  }

  textarea {
    width: 100%;
    font: inherit;
    font-family: ui-monospace, monospace;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 10px;
  }

  summary {
    cursor: pointer;
  }
</style>
