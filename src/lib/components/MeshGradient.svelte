<script lang="ts">
  // Mesh gradient animado, barato para o WebView do Android:
  // 4 camadas com radial-gradient (já suave, sem filter: blur), animadas só com transform
  // (compositor). Pausa fora da tela (IntersectionObserver) e respeita prefers-reduced-motion.
  // As cores vêm das variáveis do tema (data-lava).
  // @req RNF-05
  type Props = { class?: string; intensity?: number; children?: import('svelte').Snippet };
  let { class: cls = '', intensity = 1, children }: Props = $props();

  let el: HTMLDivElement;
  let paused = $state(true);

  $effect(() => {
    const io = new IntersectionObserver(([e]) => { paused = !e.isIntersecting; }, { threshold: 0.01 });
    io.observe(el);
    const onVis = () => { if (document.hidden) paused = true; };
    document.addEventListener('visibilitychange', onVis);
    return () => { io.disconnect(); document.removeEventListener('visibilitychange', onVis); };
  });
</script>

<div bind:this={el} class="mesh relative isolate overflow-hidden {cls}" class:paused style="background: var(--bg)">
  <div class="absolute inset-0" aria-hidden="true" style="opacity:{0.9 * intensity}">
    <div class="mesh-layer" style="left:-15%;top:-25%;background:radial-gradient(closest-side, var(--lava) 0%, transparent 100%);animation:mesh-a 18s infinite"></div>
    <div class="mesh-layer" style="right:-20%;top:-10%;width:75%;height:100%;background:radial-gradient(closest-side, var(--lava-deep) 0%, transparent 100%);animation:mesh-b 23s infinite"></div>
    <div class="mesh-layer" style="left:25%;bottom:-45%;width:70%;background:radial-gradient(closest-side, var(--lava-core) 0%, transparent 100%);opacity:.55;animation:mesh-c 27s infinite"></div>
    <div class="mesh-layer" style="left:-10%;bottom:-30%;width:60%;height:70%;background:radial-gradient(closest-side, color-mix(in oklab, var(--lava) 55%, var(--lava-ember)) 0%, transparent 100%);animation:mesh-b 31s infinite reverse"></div>
  </div>
  <!-- véu para manter o texto legível: degradê para o basalto embaixo -->
  <div class="absolute inset-0" aria-hidden="true" style="background:linear-gradient(180deg, rgb(0 0 0 / .05), var(--bg) 100%)"></div>
  <div class="relative">{@render children?.()}</div>
</div>
