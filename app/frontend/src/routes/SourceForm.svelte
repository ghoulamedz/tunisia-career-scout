<script lang="ts">
  import { page } from '$lib/stores';
  import SourceForm from '$components/sources/SourceForm.svelte';

  // Extract ID from route params for editing
  let initialData = $derived(page.params.id ? { id: page.params.id } : undefined);
</script>

<SourceForm {initialData} onSuccess={() => window.location.href = '/sources'} onCancel={() => window.location.href = '/sources'} />