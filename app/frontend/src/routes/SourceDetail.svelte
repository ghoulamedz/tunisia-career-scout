<script lang="ts">
  import { onMount } from 'svelte';
  import { page, sourcesStore, toast } from '$lib/stores';
  import { sourcesApi } from '$lib/api';
  import { cn, formatRelativeTime, getSourceTypeIcon } from '$lib/utils';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import Badge from '$components/ui/Badge.svelte';
  import ScanLogs from '$components/sources/ScanLogs.svelte';
  import Modal from '$components/ui/Modal.svelte';
  import { ChevronLeft, Edit, Play, Trash2, Database, Rss, Mail, Users, AlertCircle, CheckCircle, Loader, ToggleLeft, ToggleRight } from '@lucide/svelte';

  let source = $state<any>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let scanning = $state(false);
  let toggling = $state(false);
  let showDeleteModal = $state(false);

  async function loadSource() {
    const id = page.params.id;
    if (!id) return;

    loading = true;
    error = null;
    try {
      source = await sourcesApi.get(id);
    } catch (e) {
      error = e as string;
    } finally {
      loading = false;
    }
  }

  async function handleScan() {
    if (!source || !source.enabled || scanning) return;
    scanning = true;
    try {
      await sourcesApi.scan(source.id);
      toast('success', 'Scan started');
      await loadSource();
    } catch (e) {
      toast('error', 'Failed to start scan');
    } finally {
      scanning = false;
    }
  }

  async function handleToggle() {
    if (!source || toggling) return;
    toggling = true;
    try {
      await sourcesApi.toggleEnabled(source.id);
      toast('success', source.enabled ? 'Source disabled' : 'Source enabled');
      await loadSource();
    } catch (e) {
      toast('error', 'Failed to toggle source');
    } finally {
      toggling = false;
    }
  }

  function confirmDelete() {
    showDeleteModal = true;
  }

  async function handleDelete() {
    if (!source) return;
    try {
      await sourcesApi.delete(source.id);
      toast('success', 'Source deleted');
      window.location.href = '/sources';
    } catch (e) {
      toast('error', 'Failed to delete source');
    } finally {
      showDeleteModal = false;
    }
  }

  function getIcon(type: string) {
    switch (type) {
      case 'url': return Database;
      case 'rss': return Rss;
      case 'gmail': return Mail;
      case 'social': return Users;
      default: return Database;
    }
  }

  let Icon = $derived.by(() => source ? getIcon(source.source_type) : Database);
  let iconColor = $derived(source ? (source.enabled ? 'var(--color-primary-600)' : 'var(--color-secondary-500)') : 'var(--color-secondary-500)');

  function getStatusIcon(status: string) {
    switch (status) {
      case 'completed': return CheckCircle;
      case 'failed': return AlertCircle;
      case 'running': return Loader;
      default: return AlertCircle;
    }
  }

  onMount(() => loadSource());
</script>

<div class="max-w-4xl mx-auto space-y-6">
  <div class="flex items-center justify-between">
    <a href="/sources" class="flex items-center gap-2 text-secondary-600 hover:text-secondary-900 dark:text-secondary-400 dark:hover:text-white">
      <ChevronLeft class="h-5 w-5" />
      Back to Sources
    </a>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <Loader class="h-8 w-8 animate-spin text-primary-600" />
    </div>
  {:else if error}
    <Card class="border-red-200 dark:border-red-800">
      <div class="p-6 text-center">
        <AlertCircle class="h-12 w-12 mx-auto text-red-500 mb-4" />
        <p class="text-red-600 dark:text-red-400 mb-4">{error}</p>
        <a href="/sources"><Button variant="primary">Back to Sources</Button></a>
      </div>
    </Card>
  {:else if source}
    <!-- Source Header -->
    <Card>
      <div class="p-6">
        <div class="flex items-start justify-between gap-4">
          <div class="flex items-center gap-4 flex-1 min-w-0">
            <div class={cn('p-3 rounded-xl', source.enabled ? 'bg-primary-100 dark:bg-primary-900/30' : 'bg-secondary-100 dark:bg-secondary-800')}>
              <span style:color={iconColor}><Icon class="h-8 w-8" /></span>
            </div>
            <div class="min-w-0">
              <div class="flex items-center gap-3">
                <h1 class="text-2xl font-bold text-secondary-900 dark:text-white truncate">{source.name}</h1>
                <Badge variant={source.enabled ? 'success' : 'secondary'}>{source.enabled ? 'Enabled' : 'Disabled'}</Badge>
              </div>
              <div class="flex items-center gap-4 mt-2 text-sm text-secondary-600 dark:text-secondary-400">
                <span class="flex items-center gap-1">
                  <span class="capitalize">{source.source_type}</span>
                </span>
                <span>Schedule: <code class="font-mono text-xs bg-secondary-100 dark:bg-secondary-800 px-1.5 py-0.5 rounded">{source.schedule}</code></span>
                <span>Status: <Badge variant={source.status === 'active' ? 'success' : source.status === 'paused' ? 'secondary' : 'destructive'}>{source.status}</Badge></span>
              </div>
            </div>
          </div>
          <div class="flex items-center gap-2 flex-shrink-0">
            <Button variant="secondary" class="" on:click={() => window.location.href = `/sources/edit/${source.id}`}>
              <Edit class="h-4 w-4 mr-2" />
              Edit
            </Button>
            <Button
              variant={source.enabled ? 'primary' : 'outline'}
              class="whitespace-nowrap"
              on:click={handleScan}
              disabled={scanning || !source.enabled}
            >
              {#if scanning}
                <Loader class="h-4 w-4 mr-2 animate-spin" />
                Scanning...
              {:else}
                <Play class="h-4 w-4 mr-2" />
                Scan Now
              {/if}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class={source.enabled ? 'text-green-600 hover:bg-green-50 dark:hover:bg-green-900/20' : 'text-secondary-500 hover:bg-secondary-100 dark:hover:bg-secondary-800'}
              on:click={handleToggle}
              disabled={toggling}
              title={source.enabled ? 'Disable source' : 'Enable source'}
            >
              {#if toggling}
                <Loader class="h-4 w-4 animate-spin" />
              {:else if source.enabled}
                <ToggleRight class="h-4 w-4" />
              {:else}
                <ToggleLeft class="h-4 w-4" />
              {/if}
            </Button>
            <Button variant="ghost" size="sm" class="text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20" on:click={confirmDelete}>
              <Trash2 class="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div class="mt-6 pt-6 border-t border-secondary-200 dark:border-secondary-700 grid gap-4 md:grid-cols-4">
          <div>
            <p class="text-xs text-secondary-500 uppercase tracking-wide">Created</p>
            <p class="font-mono text-sm text-secondary-900 dark:text-white">{formatRelativeTime(source.created_at)}</p>
          </div>
          <div>
            <p class="text-xs text-secondary-500 uppercase tracking-wide">Updated</p>
            <p class="font-mono text-sm text-secondary-900 dark:text-white">{formatRelativeTime(source.updated_at)}</p>
          </div>
          {#if source.last_scan_at}
            <div>
              <p class="text-xs text-secondary-500 uppercase tracking-wide">Last Scan</p>
              <p class="font-mono text-sm text-secondary-900 dark:text-white">{formatRelativeTime(source.last_scan_at)}</p>
            </div>
          {:else}
            <div>
              <p class="text-xs text-secondary-500 uppercase tracking-wide">Last Scan</p>
              <p class="font-mono text-sm text-secondary-500 dark:text-secondary-400">Never</p>
            </div>
          {/if}
          <div>
            <p class="text-xs text-secondary-500 uppercase tracking-wide">ID</p>
            <p class="font-mono text-xs text-secondary-500 dark:text-secondary-400 truncate max-w-xs">{source.id}</p>
          </div>
        </div>
      </div>
    </Card>

    <!-- Source Configuration -->
    <Card>
      <div class="p-6">
        <h2 class="text-lg font-semibold text-secondary-900 dark:text-white mb-4">Configuration</h2>
        <div class="bg-secondary-50 dark:bg-secondary-800/50 rounded-lg p-4 font-mono text-sm overflow-x-auto">
          <pre class="text-secondary-700 dark:text-secondary-300">{JSON.stringify(source.config, null, 2)}</pre>
        </div>
      </div>
    </Card>

    <!-- Scan Logs -->
    <ScanLogs sourceId={source.id} limit={20} />
  {/if}
</div>

<Modal open={showDeleteModal} title="Delete Source" on:close={() => showDeleteModal = false}>
  <p class="text-secondary-600 dark:text-secondary-400 mb-6">Are you sure you want to delete <strong>{source?.name}</strong>? This action cannot be undone.</p>
  <div class="flex justify-end gap-3">
    <Button variant="secondary" class="" on:click={() => showDeleteModal = false}>Cancel</Button>
    <Button variant="destructive" class="" on:click={handleDelete} disabled={!source}>Delete</Button>
  </div>
</Modal>