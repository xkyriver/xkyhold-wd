<script lang="ts">
  // The configured backup's key is not in the Credential Manager: ask for the password.
  import { t } from '../lib/i18n.svelte'
  import { app, explain, unlock } from '../state/app.svelte'

  let password = $state('')
  let recoveryInput = $state('')
  let useRecoveryKey = $state(false)
  let remember = $state(true)
  let busy = $state(false)
  let error = $state<string | null>(null)

  async function open() {
    busy = true
    error = null
    try {
      await unlock(useRecoveryKey ? { recovery_key: recoveryInput } : { password }, remember && !!app.status?.keyring)
      password = recoveryInput = ''
    } catch (err) {
      error = explain(err)
    } finally {
      busy = false
    }
  }
</script>

<section class="card stack">
  <h1>{t('Open your backup')}</h1>
  <p class="muted">{t('This computer does not keep the key of your backup. Type its password to open it.')}</p>
  {#if useRecoveryKey}
    <div>
      <label for="rk">{t('Recovery key')}</label>
      <input id="rk" type="text" bind:value={recoveryInput} autocomplete="off" spellcheck="false" />
    </div>
  {:else}
    <div>
      <label for="pw">{t('Password')}</label>
      <input id="pw" type="password" bind:value={password} autocomplete="current-password" onkeydown={(e) => e.key === 'Enter' && open()} />
    </div>
  {/if}
  <button class="link small" onclick={() => (useRecoveryKey = !useRecoveryKey)}>
    {useRecoveryKey ? t('Use my password instead') : t('I forgot my password')}
  </button>
  {#if app.status?.keyring}
    <label class="check"><input type="checkbox" bind:checked={remember} /> {t('Open this backup on this computer without the password')}</label>
  {/if}
  <div class="row end">
    <button class="btn primary" onclick={open} disabled={busy || !(useRecoveryKey ? recoveryInput.trim() : password)}>
      {busy ? t('Opening your backup…') : t('Open backup')}
    </button>
  </div>
  {#if error}<div class="banner warn" role="alert"><span>{error}</span></div>{/if}
</section>

<style>
  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 400;
    cursor: pointer;
  }
</style>
