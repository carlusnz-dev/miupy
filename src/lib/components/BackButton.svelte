<script lang="ts">
  // @req RF-43
  import { router } from '../stores/router.svelte';
  import Icon from './Icon.svelte';
</script>

<button type="button" class="btn btn-sm inline-flex items-center gap-1" onclick={() => router.back()}>
  <Icon name="left" size={16} />Voltar
</button>
