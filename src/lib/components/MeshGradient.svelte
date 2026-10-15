<script lang="ts">
  // Mesh gradient animado, leve para o WebView do Android e o WebKitGTK com GPU:
  // 3 camadas com radial-gradient (já suave, sem filter: blur), animadas só com transform
  // (compositor), devagar. Pausa fora da tela (IntersectionObserver), com a aba oculta e com a
  // janela sem foco; `will-change` só vale enquanto anima. Respeita prefers-reduced-motion.
  // As cores vêm das variáveis do tema (data-lava).
  // @req RNF-05 RNF-09 RNF-13
  type Props = { class?: string; intensity?: number; children?: import('svelte').Snippet };
  let { class: cls = '', intensity = 1, children }: Props = $props();

  let el: HTMLDivElement;
  let offscreen = $state(true);
  let inactive = $state(typeof document !== 'undefined' && (document.hidden || !document.hasFocus()));
  const paused = $derived(offscreen || inactive);

  $effect(() => {
    const io = new IntersectionObserver(([e]) => { offscreen = !e.isIntersecting; }, { threshold: 0.01 });
    io.observe(el);
    const sync = () => { inactive = document.hidden || !document.hasFocus(); };
    document.addEventListener('visibilitychange', sync);
    window.addEventListener('blur', sync);
    window.addEventListener('focus', sync);
    return () => {
      io.disconnect();
      document.removeEventListener('visibilitychange', sync);
      window.removeEventListener('blur', sync);
      window.removeEventListener('focus', sync);
    };
  });
</script>

<div bind:this={el} class="mesh relative isolate overflow-hidden {cls}" class:paused style="background: var(--bg)">
  <div class="absolute inset-0" aria-hidden="true" style="opacity:{0.9 * intensity}">
    <div class="mesh-layer" style="left:-15%;top:-25%;background:radial-gradient(closest-side, var(--lava) 0%, transparent 100%);animation:mesh-a 34s infinite"></div>
    <div class="mesh-layer" style="right:-20%;top:-10%;width:75%;height:100%;background:radial-gradient(closest-side, var(--lava-deep) 0%, transparent 100%);animation:mesh-b 44s infinite"></div>
    <div class="mesh-layer" style="left:25%;bottom:-45%;width:70%;background:radial-gradient(closest-side, var(--lava-core) 0%, transparent 100%);opacity:.55;animation:mesh-c 52s infinite"></div>
  </div>
  <!-- véu para manter o texto legível: degradê para o basalto embaixo -->
  <div class="absolute inset-0" aria-hidden="true" style="background:linear-gradient(180deg, rgb(0 0 0 / .05), var(--bg) 100%)"></div>
  <div class="relative">{@render children?.()}</div>
</div>
