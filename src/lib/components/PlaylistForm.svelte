<script lang="ts">
  // Criar/renomear playlist (modal). @req RF-06
  import { api, errorMessage, type Playlist } from '../ipc';
  import { notify } from '../stores/toast.svelte';
  import Modal from './Modal.svelte';

  type Props = { state: { playlist: Playlist | null } | null; onclose: () => void; ondone?: (p: Playlist) => void };
  let { state: st, onclose, ondone }: Props = $props();

  let name = $state('');
  let description = $state('');
  let err = $state('');
  let busy = $state(false);
  $effect(() => { if (st) { name = st.playlist?.name ?? ''; description = st.playlist?.description ?? ''; err = ''; } });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) { err = 'O nome não pode ficar vazio.'; return; }
    busy = true; err = '';
    try {
      const p = st?.playlist
        ? await api.playlistUpdate({ playlistId: st.playlist.id, name, description })
        : await api.playlistCreate({ name, description: description || undefined });
      notify(st?.playlist ? 'Playlist atualizada.' : 'Playlist criada.');
      ondone?.(p);
      onclose();
    } catch (ex) { err = errorMessage(ex); } finally { busy = false; }
  }
</script>

<Modal open={st !== null} title={st?.playlist ? 'Renomear playlist' : 'Nova playlist'} {onclose}>
  <form id="pl-form" class="flex flex-col gap-3" onsubmit={submit} novalidate>
    <div>
      <label class="mb-1 block text-sm text-muted" for="pl-name">Nome</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="pl-name" class="field" bind:value={name} maxlength="80" autofocus aria-invalid={err ? 'true' : undefined} aria-describedby={err ? 'pl-err' : undefined} />
      {#if err}<p id="pl-err" class="mt-1 text-sm text-danger" role="alert">{err}</p>{/if}
    </div>
    <div>
      <label class="mb-1 block text-sm text-muted" for="pl-desc">Descrição (opcional)</label>
      <textarea id="pl-desc" class="field" rows="2" bind:value={description} maxlength="300"></textarea>
    </div>
  </form>
  {#snippet footer()}
    <button class="btn btn-sm" type="button" onclick={onclose}>Cancelar</button>
    <button class="btn btn-sm btn-primary" form="pl-form" disabled={busy}>{st?.playlist ? 'Salvar' : 'Criar'}</button>
  {/snippet}
</Modal>
