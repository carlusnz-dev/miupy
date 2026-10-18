<script module lang="ts">
  export type MenuItem = { label: string; icon?: string; danger?: boolean; separated?: boolean; checked?: boolean; onselect: () => void };
</script>

<script lang="ts">
  // Menu de ações ("Mais opções") no tema magma. Setas, Home/End, inicial, Enter/Espaço e Escape;
  // o foco volta ao botão ao fechar. @req RNF-04 RNF-05
  import { springTransition } from '../motion';
  import { dismissable } from '../util/dismiss';
  import { listboxKey } from '../util/listbox';
  import Icon from './Icon.svelte';

  type Props = { items: MenuItem[]; label: string; disabled?: boolean; class?: string };
  let { items, label, disabled = false, class: cls = '' }: Props = $props();

  const uid = $props.id();
  let open = $state(false);
  let active = $state(0);
  let button: HTMLButtonElement;
  let menu = $state<HTMLDivElement>();
  const labels = $derived(items.map((i) => i.label));
  const itemId = (i: number) => `${uid}-i${i}`;

  function show(at = 0) {
    active = at;
    open = true;
  }

  function commit(index: number) {
    open = false;
    button.focus();
    items[index]?.onselect();
  }

  // Fechado, as setas abrem o menu; Enter e Espaço continuam sendo o clique do botão.
  function onTriggerKey(event: KeyboardEvent) {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    event.preventDefault();
    show(event.key === 'ArrowUp' ? items.length - 1 : 0);
  }

  function onMenuKey(event: KeyboardEvent) {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    if (event.key === 'Tab') { open = false; return; }
    const move = listboxKey(event.key, active, labels);
    if (move === null) return;
    event.preventDefault();
    if (move === 'commit') commit(active); else active = move;
  }

  $effect(() => {
    if (open) menu?.focus();
  });
  $effect(() => {
    if (open) document.getElementById(itemId(active))?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="relative inline-block {cls}" use:dismissable={(reason) => {
  if (!open) return;
  open = false;
  if (reason === 'escape') button.focus();
}}>
  <button
    bind:this={button} type="button" class="icon-btn" {disabled} aria-label={label} title={label}
    aria-haspopup="menu" aria-expanded={open} aria-controls={open ? `${uid}-menu` : undefined}
    onclick={() => (open ? (open = false) : show())} onkeydown={onTriggerKey}
  >
    <Icon name="more" size={20} />
  </button>
  {#if open}
    <div
      bind:this={menu} id="{uid}-menu" role="menu" aria-label={label} tabindex="-1" aria-activedescendant={itemId(active)}
      onkeydown={onMenuKey} in:springTransition={{ y: -4, duration: 180 }}
      class="glass-strong absolute right-0 z-30 mt-1 min-w-52 rounded-lg p-1 outline-none"
    >
      {#each items as item, i (item.label)}
        {#if item.separated}<div class="my-1 border-t border-line" role="separator"></div>{/if}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          id={itemId(i)} role={item.checked === undefined ? 'menuitem' : 'menuitemcheckbox'} aria-checked={item.checked} tabindex="-1"
          class="flex min-h-12 cursor-pointer items-center gap-3 whitespace-nowrap rounded-md px-3 text-sm md:min-h-8
            {i === active ? 'bg-s3' : ''} {item.danger ? 'text-danger' : ''}"
          onmousedown={(e) => e.preventDefault()} onmouseenter={() => (active = i)} onclick={() => commit(i)}
        >
          {#if item.icon}<Icon name={item.icon} size={16} />{/if}{item.label}
          {#if item.checked}<Icon name="check" size={14} class="ml-auto text-lava-text" />{/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
