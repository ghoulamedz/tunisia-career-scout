<script lang="ts">
  import { onMount } from 'svelte';
  import { postingsStore, sourcesStore, toast } from '$lib/stores';
  import { postingsApi, sourcesApi } from '$lib/api';
  import { cn, formatRelativeTime, getStatusColor } from '$lib/utils';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import Modal from '$components/ui/Modal.svelte';
  import { Briefcase, Search, Filter, ChevronLeft, ChevronRight, Trash2, RefreshCw } from '@lucide/svelte';

  let searchQuery = $state('');
  let debouncedSearch = $state('');
  let statusFilter = $state('');
  let sourceFilter = $state('');
  let currentPage = $state(1);
  let showDeleteModal = $state(false);
  let deletingPosting = $state(null as any);

  const statuses = ['active', 'expired', 'filled', 'hidden', 'applied', 'saved'];

  async function loadPostings() {
    await postingsStore.load();
  }

  async function loadSources() {
    await sourcesStore.load(true);
  }

  function handleSearch() {
    debouncedSearch = searchQuery;
    currentPage = 1;
    if (debouncedSearch) {
      postingsStore.search(debouncedSearch);
    } else {
      loadPostings();
    }
  }

  function setStatusFilter(status: string) {
    statusFilter = statusFilter === status ? '' : status;
    currentPage = 1;
    postingsStore.filters.status = statusFilter || null;
    loadPostings();
  }

  function setSourceFilter(id: string) {
    sourceFilter = sourceFilter === id ? '' : id;
    currentPage = 1;
    postingsStore.filters.sourceId = sourceFilter || null;
    loadPostings();
  }

  function confirmDelete(posting: any) {
    deletingPosting = posting;
    showDeleteModal = true;
  }

  async function handleDelete() {
    if (!deletingPosting) return;
    try {
      await postingsApi.delete(deletingPosting.id);
      toast('success', 'Posting deleted');
      loadPostings();
    } catch (e) {
      toast('error', 'Failed to delete posting');
    } finally {
      showDeleteModal = false;
      deletingPosting = null;
    }
  }

  async function handleReExtract(posting: any) {
    // Would need LLM settings from settings store
    toast('info', 'Re-extract with LLM coming soon');
  }

  onMount(() => {
    loadPostings();
    loadSources();
  });

  $effect(() => {
    const timer = setTimeout(() => {
      debouncedSearch = searchQuery;
      handleSearch();
    }, 300);
    return () => clearTimeout(timer);
  });
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between flex-wrap gap-4">
    <div>
      <h1 class="text-2xl font-bold text-secondary-900 dark:text-white">Job Postings</h1>
      <p class="text-secondary-600 dark:text-secondary-400">Browse and manage scraped job postings</p>
    </div>
    <div class="flex items-center gap-2">
      <Button variant="secondary" on:click={loadPostings} disabled={postingsStore.loading}>
        <RefreshCw class="h-4 w-4" />
        Refresh
      </Button>
    </div>
  </div>

  <!-- Filters -->
  <Card>
    <div class="p-4 space-y-4">
      <div class="flex flex-wrap gap-4">
        <div class="flex-1 min-w-[250px] relative">
          <Search class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-secondary-400" />
          <input
            bind:value={searchQuery}
            type="text"
            placeholder="Search postings..."
            class="w-full pl-10 pr-4 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          />
        </div>

        <div class="flex gap-2">
          <select
            bind:value={statusFilter}
            class="px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          >
            <option value="">All Statuses</option>
            {#each statuses as s}
              <option value={s}>{s.charAt(0).toUpperCase() + s.slice(1)}</option>
            {/each}
          </select>

          <select
            bind:value={sourceFilter}
            class="px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent min-w-[150px]"
          >
            <option value="">All Sources</option>
            {#each sourcesStore.items as source}
              <option value={source.id}>{source.name}</option>
            {/each}
          </select>
        </div>
      </div>
    </div>
  </Card>

  <!-- Postings List -->
  {#if postingsStore.loading}
    <div class="flex justify-center py-12">
      <svg class="animate-spin h-8 w-8 text-primary-600" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" /><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" /></svg>
    </div>
  {:else if postingsStore.items.length === 0}
    <Card>
      <div class="p-12 text-center">
        <Briefcase class="h-12 w-12 mx-auto text-secondary-300 dark:text-secondary-600 mb-4" />
        <h3 class="text-lg font-medium text-secondary-900 dark:text-white mb-2">No postings found</h3>
        <p class="text-secondary-600 dark:text-secondary-400 mb-6">
          {searchQuery || statusFilter || sourceFilter
            ? 'Try adjusting your filters'
            : 'Add a source and run a scan to discover job postings'}
        </p>
        <a href="/sources/new"><Button variant="primary" size="lg"><Plus class="h-4 w-4 mr-2" />Add Source</Button></a>
      </div>
    </Card>
  {:else}
    <div class="space-y-3">
      {#each postingsStore.items as posting}
        <Card variant="outlined" class="hover:shadow-md transition-shadow">
          <a href="/postings/{posting.id}" class="block p-4">
            <div class="flex items-start justify-between gap-4">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-1">
                  <h3 class="font-medium text-secondary-900 dark:text-white truncate">{posting.title}</h3>
                  <span class={cn('px-2 py-0.5 text-xs font-medium rounded-full', getStatusColor(posting.status))}>
                    {posting.status.charAt(0).toUpperCase() + posting.status.slice(1)}
                  </span>
                </div>
                <p class="text-secondary-600 dark:text-secondary-400">{posting.company}</p>
                <div class="flex flex-wrap items-center gap-3 mt-2 text-sm text-secondary-500">
                  <span>{formatRelativeTime(posting.scraped_at)}</span>
                  {#if posting.location}
                    <span class="flex items-center gap-1">
                      <svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L12 21l-5.657-5.657L12 9.657l5.657 5.657z" /></svg>
                      {posting.location}
                    </span>
                  {/if}
                  {#if posting.remote_policy}
                    <span class="px-2 py-0.5 bg-secondary-100 dark:bg-secondary-800 rounded">{posting.remote_policy}</span>
                  {/if}
                  {#if posting.job_type}
                    <span class="px-2 py-0.5 bg-secondary-100 dark:bg-secondary-800 rounded">{posting.job_type}</span>
                  {/if}
                </div>
                {#if posting.skills.length > 0}
                  <div class="flex flex-wrap gap-1 mt-2">
                    {#each posting.skills.slice(0, 5) as skill}
                      <span class="px-2 py-0.5 text-xs bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300 rounded">{skill}</span>
                    {/each}
                    {#if posting.skills.length > 5}
                      <span class="px-2 py-0.5 text-xs bg-secondary-100 dark:bg-secondary-800 text-secondary-600 dark:text-secondary-400 rounded">+{posting.skills.length - 5}</span>
                    {/if}
                  </div>
                {/if}
              </div>
              <div class="flex items-center gap-1 shrink-0">
                <Button variant="ghost" size="icon" class="text-secondary-600 hover:text-red-600" onclick={(e) => { e.preventDefault(); confirmDelete(posting); }}>
                  <Trash2 class="h-4 w-4" />
                </Button>
              </div>
            </div>
          </a>
        </Card>
      {/each}
    </div>

    <!-- Pagination -->
    {#if postingsStore.total > postingsStore.pageSize}
      <div class="flex items-center justify-center gap-2">
        <Button variant="secondary" size="sm" on:click={() => { currentPage--; loadPostings(); }} disabled={currentPage === 1}>
          <ChevronLeft class="h-4 w-4" />
        </Button>
        <span class="px-4 text-sm text-secondary-600 dark:text-secondary-400">
          Page {currentPage} of {Math.ceil(postingsStore.total / postingsStore.pageSize)}
        </span>
        <Button variant="secondary" size="sm" on:click={() => { currentPage++; loadPostings(); }} disabled={currentPage >= Math.ceil(postingsStore.total / postingsStore.pageSize)}>
          <ChevronRight class="h-4 w-4" />
        </Button>
      </div>
    {/if}
  {/if}
</div>

<Modal open={showDeleteModal} title="Delete Posting" on:close={() => showDeleteModal = false}>
  <p class="text-secondary-600 dark:text-secondary-400 mb-6">Are you sure you want to delete <strong>{deletingPosting?.title}</strong>? This action cannot be undone.</p>
  <div class="flex justify-end gap-3">
    <Button variant="secondary" on:click={() => showDeleteModal = false}>Cancel</Button>
    <Button variant="destructive" on:click={handleDelete} disabled={!deletingPosting}>Delete</Button>
  </div>
</Modal>