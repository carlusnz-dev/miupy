<script lang="ts">
  const keys = [['Espaço', 'tocar/pausar'], ['← →', 'voltar/avançar 5 s'], ['↑ ↓', 'volume'], ['M', 'silenciar']];
</script>

<section aria-labelledby="h-keys" class="hidden md:block">
  <h2 id="h-keys" class="mb-2 text-lg font-semibold">Atalhos de teclado</h2>
  <dl class="card grid max-w-md grid-cols-[auto_1fr] gap-x-6 gap-y-2 p-4 text-sm">
    {#each keys as [k, d]}<dt><kbd class="chip">{k}</kbd></dt><dd class="text-muted">{d}</dd>{/each}
  </dl>
</section>
