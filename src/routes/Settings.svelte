<script lang="ts">
  // Configurações: uma seção por componente em lib/components/settings/, para frentes
  // diferentes não editarem o mesmo arquivo. @req RF-17
  import { isMock } from '../lib/ipc';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import LavaSection from '../lib/components/settings/LavaSection.svelte';
  import YtDlpSection from '../lib/components/settings/YtDlpSection.svelte';
  import GpuSection from '../lib/components/settings/GpuSection.svelte';
  import MetadataSection from '../lib/components/settings/MetadataSection.svelte';
  import KeysSection from '../lib/components/settings/KeysSection.svelte';
</script>

<PageHeader title="Configurações" />

<LavaSection />
<YtDlpSection />
<GpuSection />
<MetadataSection />
<KeysSection />

{#if isMock}
  <p class="mt-8 text-xs text-faint">Rodando com dados de demonstração (mock em memória). Dentro do app Tauri, os dados vêm do backend.</p>
{/if}
