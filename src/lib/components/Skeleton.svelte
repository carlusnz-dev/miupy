<script lang="ts">
  // @req RNF-11 RNF-04
  // Bloco de esqueleto: forma chapada com pulso discreto de opacidade (só compositor). Sem
  // `prefers-reduced-motion` o pulso para e o bloco fica estático.
  type Props = { w?: string; h?: string; rounded?: string; class?: string };
  let { w = '100%', h = '0.875rem', rounded = '0.375rem', class: cls = '' }: Props = $props();
</script>

<span class="sk {cls}" style="width:{w};height:{h};border-radius:{rounded}" aria-hidden="true"></span>

<style>
  .sk {
    display: block;
    flex-shrink: 0;
    background: var(--surface-3);
    animation: sk-pulse 1.6s ease-in-out infinite;
  }
  @keyframes sk-pulse {
    0%, 100% { opacity: 0.5; }
    50% { opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .sk { animation: none; opacity: 0.75; }
  }
</style>
