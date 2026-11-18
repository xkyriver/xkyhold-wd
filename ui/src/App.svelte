<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'

  type Probe = { os: string; arch: string; user_folder: string | null; app_version: string }

  let probe = $state<Probe | null>(null)
  let error = $state('')

  invoke<Probe>('probe').then((p) => (probe = p)).catch((e) => (error = String(e)))
</script>

<main class="card" style="max-width: 520px; margin: 48px auto; padding: 28px">
  <h1 style="margin-top: 0">XkyHold-wd</h1>
  <p class="muted">Phase 0 prototype: the window reaches the Rust side.</p>
  {#if probe}
    <ul>
      <li>System: {probe.os} ({probe.arch})</li>
      <li>User folder: {probe.user_folder ?? 'unknown'}</li>
      <li>Version: {probe.app_version}</li>
    </ul>
  {:else if error}
    <p style="color: var(--danger)">{error}</p>
  {:else}
    <p>Asking Rust…</p>
  {/if}
</main>
