<script lang="ts">
  import { onMount } from 'svelte';
  import { cvApi } from '$lib/api';
  import { page, toast } from '$lib/stores';
  import CvPreview from '$components/CvPreview.svelte';
  import { ChevronLeft } from '@lucide/svelte';

  let cvId = $derived(page.params.id || '');
  let pdfData = $state<ArrayBuffer | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function loadPreview() {
    if (!cvId) {
      error = 'No CV ID provided';
      loading = false;
      return;
    }
    try {
      const pdf = await cvApi.generatePDF(cvId);
      pdfData = pdf;
    } catch (e) {
      console.error('Failed to generate PDF:', e);
      error = 'Failed to generate PDF preview';
    } finally {
      loading = false;
    }
  }

  onMount(loadPreview);
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-4">
      <a href="/cv" class="flex items-center gap-2 text-secondary-600 hover:text-secondary-900 dark:text-secondary-400 dark:hover:text-white">
        <ChevronLeft class="h-5 w-5" />
      </a>
      <div>
        <h1 class="text-2xl font-bold text-secondary-900 dark:text-white">CV Preview</h1>
        <p class="text-secondary-600 dark:text-secondary-400">Preview your CV before downloading</p>
      </div>
    </div>
  </div>

  <CvPreview {pdfData} {loading} {error} />
</div>