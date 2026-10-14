<script lang="ts">
  // Seletor de playlist e confirmação globais (controlados por stores/ui). @req RF-06
  import { api, errorMessage } from '../ipc';
  import { ui } from '../stores/ui.svelte';
  import { library } from '../stores/library.svelte';
  import { notify, notifyError } from '../stores/toast.svelte';
  import Modal from './Modal.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';

  let newName = $state('');
  let err = $state('');
  let busy = $state(false);

  $effect(() => { if (ui.picker) { newName = ''; err = ''; void library.reloadPlaylists(); } });

  async function addTo(playlistId: string, name: string) {
    if (!ui.picker) return;
    busy = true;
    try {
      await api.playlistAddTracks({ playlistId, trackIds: ui.picker.trackIds });
      const n = ui.picker.trackIds.length;
      notify(`${n} ${n === 1 ? 'faixa adicionada' : 'faixas adicionadas'} a "${name}".`);
      ui.picker = null;
    } catch (e) { notifyError(e); } finally { busy = false; }
  }
  async function createAndAdd(ev: SubmitEvent) {
    ev.preventDefault();
    if (!newName.trim()) { err = 'Dê um nome à playlist.'; return; }
    busy = true; err = '';
    try {
      const p = await api.playlistCreate({ name: newName });
      await addTo(p.id, p.name);
    } catch (e) { err = errorMessage(e); busy = false; }
  }
</script>

<Modal open={ui.picker !== null} title="Adicionar à playlist" onclose={() => (ui.picker = null)}>
  <form class="mb-4 flex gap-2" onsubmit={createAndAdd}>
    <label class="sr-only" for="np-name">Nome da nova playlist</label>
    <input id="np-name" class="field" placeholder="Nova playlist…" bind:value={newName} aria-invalid={err ? 'true' : undefined} />
    <button class="btn btn-primary shrink-0" disabled={busy}><Icon name="plus" size={16} />Criar</button>
  </form>
  {#if err}<p class="-mt-2 mb-3 text-sm text-danger" role="alert">{err}</p>{/if}
  {#if library.playlists.length === 0}
    <p class="py-4 text-center text-sm text-muted">Você ainda não tem playlists.</p>
  {:else}
    <ul class="flex flex-col gap-1">
      {#each library.playlists as p (p.id)}
        <li>
          <button class="flex w-full items-center gap-3 rounded-lg p-2 text-left hover:bg-s3" disabled={busy} onclick={() => addTo(p.id, p.name)}>
            <Cover path={p.coverPath} seed={p.id} size={40} />
            <span class="min-w-0 flex-1">
              <span class="block truncate font-medium">{p.name}</span>
              <span class="block text-xs text-muted">{p.trackCount} faixas</span>
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</Modal>

<Modal open={ui.confirmState !== null} title={ui.confirmState?.title ?? ''} onclose={() => ui.answer(false)}>
  <p class="text-sm text-muted">{ui.confirmState?.text}</p>
  {#snippet footer()}
    <button class="btn btn-sm" onclick={() => ui.answer(false)}>Cancelar</button>
    <button class="btn btn-sm {ui.confirmState?.danger ? 'btn-danger' : 'btn-primary'}" onclick={() => ui.answer(true)}>{ui.confirmState?.confirmLabel}</button>
  {/snippet}
</Modal>
