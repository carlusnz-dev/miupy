<script lang="ts">
  // O mesh é estático por padrão; só Tocando opta pela animação das camadas.
  // As cores da capa continuam transicionando pelas variáveis registradas em app.css.
  // @req RNF-05 RNF-09 RNF-13 RF-40
  type Props = { class?: string; intensity?: number; colors?: string[]; animated?: boolean; children?: import('svelte').Snippet };
  let { class: cls = '', intensity = 1, colors, animated = false, children }: Props = $props();
  let inactive = $state(true);

  $effect(() => {
    if (!animated) return;
    const sync = () => { inactive = document.hidden || !document.hasFocus(); };
    sync();
    document.addEventListener('visibilitychange', sync);
    window.addEventListener('blur', sync);
    window.addEventListener('focus', sync);
    return () => {
      document.removeEventListener('visibilitychange', sync);
      window.removeEventListener('blur', sync);
      window.removeEventListener('focus', sync);
    };
  });
</script>

<div class="mesh relative isolate overflow-hidden {cls}" class:mesh-animated={animated} class:paused={animated && inactive} style="background: var(--bg)" style:--mesh-1={colors?.[0]} style:--mesh-2={colors?.[1]} style:--mesh-3={colors?.[2]} style:--mesh-dur={colors ? '1000ms' : undefined}>
  <div class="absolute inset-0" aria-hidden="true" style="opacity:{0.9 * intensity}">
    <div class="mesh-layer" style="left:-15%;top:-25%;background:radial-gradient(closest-side, var(--mesh-1) 0%, transparent 100%);transform:translate3d(1%, 1%, 0) scale(1.09)"></div>
    <div class="mesh-layer" style="right:-20%;top:-10%;width:75%;height:100%;background:radial-gradient(closest-side, var(--mesh-2) 0%, transparent 100%);transform:translate3d(-1%, 1%, 0) scale(1.02)"></div>
    <div class="mesh-layer" style="left:25%;bottom:-45%;width:70%;background:radial-gradient(closest-side, var(--mesh-3) 0%, transparent 100%);opacity:.55;transform:translate3d(3%, 0, 0) scale(1.12)"></div>
  </div>
  <!-- véu para manter o texto legível: degradê para o basalto embaixo -->
  <div class="absolute inset-0" aria-hidden="true" style="background:linear-gradient(180deg, rgb(0 0 0 / .05), var(--bg) 100%)"></div>
  <div class="relative">{@render children?.()}</div>
</div>
