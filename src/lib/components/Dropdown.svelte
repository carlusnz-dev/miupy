<script lang="ts" generics="T extends string | number">
  // Seletor no tema magma no lugar do <select> nativo. @req RNF-04 RNF-05
  import { springTransition } from '../motion';
  import { dismissable } from '../util/dismiss';
  import { listboxKey } from '../util/listbox';
  import Icon from './Icon.svelte';

  type Props = {
    options: { value: T; label: string }[];
    value: T;
    /** Vai no `<label for>` ou no `aria-label`; o botão recebe este id. */
    id?: string;
    label?: string;
    class?: string;
    onchange?: (value: T) => void;
  };
  let { options, value = $bindable(), id, label, class: cls = '', onchange }: Props = $props();

  const uid = $props.id();
  let open = $state(false);
  let active = $state(0);
  let button: HTMLButtonElement;

  const labels = $derived(options.map((o) => o.label));
  const selected = $derived(options.findIndex((o) => o.value === value));
  const optionId = (i: number) => `${uid}-o${i}`;

  function show() {
    active = Math.max(0, selected);
    open = true;
  }

  function commit(index: number) {
    open = false;
    button.focus();
    if (index < 0 || options[index].value === value) return;
    // Com onchange o pai é dono do valor (e pode recusar a troca); sem ele, vale o bind.
    if (onchange) onchange(options[index].value); else value = options[index].value;
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    if (event.key === 'Tab') { open = false; return; }
    const move = listboxKey(event.key, open ? active : Math.max(0, selected), labels);
    if (move === null) return;
    event.preventDefault();
    if (!open) { show(); if (typeof move === 'number') active = move; return; }
    if (move === 'commit') commit(active); else active = move;
  }

  $effect(() => {
    if (open) document.getElementById(optionId(active))?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="relative inline-block {cls}" use:dismissable={(reason) => {
  if (!open) return;
  open = false;
  if (reason === 'escape') button.focus();
}}>
  <button
    bind:this={button} {id} type="button" role="combobox" class="field flex items-center justify-between gap-2 text-left"
    aria-label={label} aria-haspopup="listbox" aria-expanded={open} aria-controls={open ? `${uid}-list` : undefined}
    aria-activedescendant={open ? optionId(active) : undefined}
    onclick={() => (open ? (open = false) : show())} {onkeydown}
    onkeyup={(e) => { if (e.key === ' ') e.preventDefault(); }}
  >
    <span class="truncate">{options[selected]?.label ?? ''}</span>
    <Icon name="down" size={16} class="shrink-0 text-faint" />
  </button>
  {#if open}
    <ul
      id="{uid}-list" role="listbox" aria-label={label} in:springTransition={{ y: -4, duration: 180 }}
      class="absolute left-0 z-30 mt-1 max-h-64 min-w-full overflow-auto rounded-lg border border-line-strong bg-s2 p-1"
    >
      {#each options as option, i (option.value)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li
          id={optionId(i)} role="option" aria-selected={i === selected}
          class="flex min-h-8 cursor-pointer items-center justify-between gap-3 whitespace-nowrap rounded-md px-3 text-sm
            {i === active ? 'bg-s3' : ''} {i === selected ? 'text-lava-text' : ''}"
          onmousedown={(e) => e.preventDefault()} onmouseenter={() => (active = i)} onclick={() => commit(i)}
        >
          {option.label}
          {#if i === selected}<Icon name="check" size={14} />{/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>
