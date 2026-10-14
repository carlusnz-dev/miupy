<script lang="ts">
  // Estados de vazio, carregando e erro, num componente só.
  import Icon from './Icon.svelte';
  type Props = {
    kind: 'loading' | 'empty' | 'error';
    title?: string; text?: string; icon?: string;
    onretry?: () => void;
    children?: import('svelte').Snippet;
  };
  let { kind, title, text, icon = 'music', onretry, children }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center gap-3 px-6 py-14 text-center" role={kind === 'error' ? 'alert' : kind === 'loading' ? 'status' : undefined}>
  {#if kind === 'loading'}
    <div class="h-8 w-8 animate-spin rounded-full border-2 border-line-strong border-t-lava" aria-hidden="true"></div>
    <p class="text-sm text-muted">{title ?? 'Carregando…'}</p>
  {:else}
    <div class="grid h-12 w-12 place-items-center rounded-full border border-line-strong {kind === 'error' ? 'text-danger' : 'text-lava-text'}">
      <Icon name={kind === 'error' ? 'alert' : icon} size={22} />
    </div>
    <h3 class="text-base font-semibold">{title ?? (kind === 'error' ? 'Algo deu errado' : 'Nada por aqui')}</h3>
    {#if text}<p class="max-w-sm text-sm text-muted">{text}</p>{/if}
    {#if kind === 'error' && onretry}
      <button class="btn btn-sm" onclick={onretry}><Icon name="refresh" size={16} />Tentar de novo</button>
    {/if}
    {@render children?.()}
  {/if}
</div>
