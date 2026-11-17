<script lang="ts">
  import { onMount } from 'svelte';
  import { sourcesStore, toast } from '$lib/stores';
  import { sourcesApi } from '$lib/api';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import Modal from '$components/ui/Modal.svelte';
  import SourceCard from '$components/sources/SourceCard.svelte';
  import Plus from '@lucide/svelte/icons/plus';
  import Database from '@lucide/svelte/icons/database';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import ToggleLeft from '@lucide/svelte/icons/toggle-left';
  import ToggleRight from '@lucide/svelte/icons/toggle-right';

  let showDeleteModal = $state(false);
  let deletingSource = $state(null as any);
  let scanningAll = $state(false);

  async function handleScan(sourceId: string) {
    try {
      await sourcesApi.scan(sourceId);
      toast('success', 'Scan started');
      sourcesStore.load();
    } catch (e) {
      toast('error', 'Failed to start scan');
    }
  }

  async function handleScanAll() {
    scanningAll = true;
    try {
      await sourcesApi.scanAll();
      toast('success', 'Scan all started');
      sourcesStore.load();
    } catch (e) {
      toast('error', 'Failed to start scan all');
    } finally {
      scanningAll = false;
    }
  }

  async function handleToggleEnabled(sourceId: string, currentEnabled: boolean) {
    try {
      await sourcesApi.toggleEnabled(sourceId);
      toast('success', currentEnabled ? 'Source disabled' : 'Source enabled');
      sourcesStore.load();
    } catch (e) {
      toast('error', 'Failed to toggle source');
    }
  }

  function confirmDelete(source: any) {
    deletingSource = source;
    showDeleteModal = true;
  }

  async function handleDelete() {
    if (!deletingSource) return;
    try {
      await sourcesApi.delete(deletingSource.id);
      toast('success', 'Source deleted');
      sourcesStore.load();
    } catch (e) {
      toast('error', 'Failed to delete source');
    } finally {
      showDeleteModal = false;
      deletingSource = null;
    }
  }

  function navigateToDetail(id: string) {
    window.location.href = `/sources/${id}`;
  }

  function navigateToEdit(id: string) {
    window.location.href = `/sources/edit/${id}`;
  }

  async function handleDeleteFromCard(sourceId: string) {
    try {
      await sourcesApi.delete(sourceId);
      toast('success', 'Source deleted');
      sourcesStore.load();
    } catch (e) {
      toast('error', 'Failed to delete source');
    }
  }

  onMount(() => sourcesStore.load());
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between flex-wrap gap-4">
    <div>
      <h1 class="text-2xl font-bold text-secondary-900 dark:text-white">Sources</h1>
      <p class="text-secondary-600 dark:text-secondary-400">Manage your job sources</p>
    </div>
    <div class="flex items-center gap-3">
      <a href="/sources/new">
        <Button variant="primary">
          <Plus class="h-4 w-4" />
          Add Source
        </Button>
      </a>
      <Button
        variant="secondary"
        class="whitespace-nowrap"
        on:click={handleScanAll}
        disabled={scanningAll || sourcesStore.items.length === 0}
      >
        {#if scanningAll}
          <RotateCcw class="h-4 w-4 mr-2 animate-spin" />
          Scanning All...
        {:else}
          <RotateCcw class="h-4 w-4 mr-2" />
          Scan All
        {/if}
      </Button>
    </div>
  </div>

  {#if sourcesStore.loading}
    <div class="flex justify-center py-12">
      <svg class="animate-spin h-8 w-8 text-primary-600" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" /><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" /></svg>
    </div>
  {:else if sourcesStore.error}
    <Card class="border-red-200 dark:border-red-800">
      <div class="p-6 text-center">
        <p class="text-red-600 dark:text-red-400">Failed to load sources: {sourcesStore.error}</p>
        <Button variant="primary" class="mt-4" on:click={() => sourcesStore.load()}>Retry</Button>
      </div>
    </Card>
  {:else if sourcesStore.items.length === 0}
    <Card>
      <div class="p-12 text-center">
        <Database class="h-12 w-12 mx-auto text-secondary-300 dark:text-secondary-600 mb-4" />
        <h3 class="text-lg font-medium text-secondary-900 dark:text-white mb-2">No sources configured</h3>
        <p class="text-secondary-600 dark:text-secondary-400 mb-6">Add your first job source to start discovering opportunities</p>
        <a href="/sources/new"><Button variant="primary" size="lg"><Plus class="h-4 w-4 mr-2" />Add Source</Button></a>
      </div>
    </Card>
  {:else}
    <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {#each sourcesStore.items as source}
        <SourceCard
          {source}
          onScan={handleScan}
          onToggle={handleToggleEnabled}
          onView={navigateToDetail}
          onEdit={navigateToEdit}
          onDelete={handleDeleteFromCard}
        />
      {/each}
    </div>
  {/if}
</div>

<Modal open={showDeleteModal} title="Delete Source" on:close={() => showDeleteModal = false}>
  <p class="text-secondary-600 dark:text-secondary-400 mb-6">Are you sure you want to delete <strong>{deletingSource?.name}</strong>? This action cannot be undone.</p>
  <div class="flex justify-end gap-3">
    <Button variant="secondary" on:click={() => showDeleteModal = false}>Cancel</Button>
    <Button variant="destructive" on:click={handleDelete} disabled={!deletingSource}>Delete</Button>
  </div>
</Modal>