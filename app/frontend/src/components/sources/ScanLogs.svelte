<script lang="ts">
  import { onMount } from 'svelte';
  import { cn, formatRelativeTime, formatDuration } from '$lib/utils';
  import { sourcesApi } from '$lib/api';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import type { ScanLog, ScanStatus } from '$lib/types';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import AlertCircle from '@lucide/svelte/icons/alert-circle';
  import CheckCircle from '@lucide/svelte/icons/check-circle';
  import Loader from '@lucide/svelte/icons/loader';

  interface Props {
    sourceId: string;
    limit?: number;
  }

  let { sourceId, limit = 50 }: Props = $props();

  let logs = $state<ScanLog[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  async function loadLogs() {
    loading = true;
    error = null;
    try {
      logs = await sourcesApi.getLogs(sourceId, limit);
    } catch (e) {
      error = e as string;
    } finally {
      loading = false;
    }
  }

  function getStatusIcon(status: ScanStatus) {
    switch (status) {
      case 'completed': return CheckCircle;
      case 'failed': return AlertCircle;
      case 'running': return Loader;
      case 'partial': return AlertCircle;
      default: return AlertCircle;
    }
  }

  function getStatusColor(status: ScanStatus) {
    switch (status) {
      case 'completed': return 'text-green-600 dark:text-green-400';
      case 'failed': return 'text-red-600 dark:text-red-400';
      case 'running': return 'text-blue-600 dark:text-blue-400 animate-spin';
      case 'partial': return 'text-amber-600 dark:text-amber-400';
      default: return 'text-secondary-600 dark:text-secondary-400';
    }
  }

  function getStatusBg(status: ScanStatus) {
    switch (status) {
      case 'completed': return 'bg-green-100 dark:bg-green-900/30';
      case 'failed': return 'bg-red-100 dark:bg-red-900/30';
      case 'running': return 'bg-blue-100 dark:bg-blue-900/30';
      case 'partial': return 'bg-amber-100 dark:bg-amber-900/30';
      default: return 'bg-secondary-100 dark:bg-secondary-800';
    }
  }

  onMount(() => loadLogs());
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h3 class="text-lg font-semibold text-secondary-900 dark:text-white">Scan History</h3>
    <Button variant="ghost" size="sm" class="" on:click={loadLogs} disabled={loading}>
      <RefreshCw class={cn('h-4 w-4', loading && 'animate-spin')} />
    </Button>
  </div>

  {#if loading && logs.length === 0}
    <div class="flex justify-center py-8">
      <Loader class="h-8 w-8 animate-spin text-primary-600" />
    </div>
  {:else if error}
    <Card class="border-red-200 dark:border-red-800">
      <div class="p-4 text-center">
        <p class="text-red-600 dark:text-red-400">Failed to load scan logs: {error}</p>
        <Button variant="outline" class="mt-2" on:click={loadLogs}>Retry</Button>
      </div>
    </Card>
  {:else if logs.length === 0}
    <Card>
      <div class="p-8 text-center">
        <AlertCircle class="h-12 w-12 mx-auto text-secondary-300 dark:text-secondary-600 mb-4" />
        <h4 class="text-secondary-900 dark:text-white mb-2">No scan history</h4>
        <p class="text-secondary-600 dark:text-secondary-400">Run a scan to see results here</p>
      </div>
    </Card>
  {:else}
    <div class="space-y-3">
      {#each logs as log}
        <Card variant="outlined" class="hover:shadow-sm transition-shadow">
          <div class="p-4">
            <div class="flex items-start justify-between gap-4">
              <div class="flex items-center gap-3 flex-1 min-w-0">
                <div class={cn('p-2 rounded-lg flex-shrink-0', getStatusBg(log.status))}>
                  <span style:color={getStatusColor(log.status)}>
                    <svelte:component this={getStatusIcon(log.status)} class="h-5 w-5" />
                  </span>
                </div>
                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="font-medium text-secondary-900 dark:text-white capitalize">{log.status}</span>
                    <span class={cn('text-xs px-2 py-0.5 rounded-full', getStatusBg(log.status), getStatusColor(log.status))}>
                      {formatRelativeTime(log.started_at)}
                    </span>
                  </div>
                  <div class="flex items-center gap-4 text-sm text-secondary-600 dark:text-secondary-400 mt-1">
                    <span>Found: <strong>{log.postings_found}</strong></span>
                    <span>New: <strong class="text-green-600 dark:text-green-400">{log.postings_new}</strong></span>
                    <span>Updated: <strong class="text-blue-600 dark:text-blue-400">{log.postings_updated}</strong></span>
                    {#if log.duration_ms}
                      <span>Duration: <strong>{formatDuration(log.duration_ms)}</strong></span>
                    {/if}
                  </div>
                </div>
              </div>
            </div>

            {#if log.error_message}
              <div class="mt-3 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                <div class="flex items-center gap-2 text-red-700 dark:text-red-300 mb-1">
                  <AlertCircle class="h-4 w-4" />
                  <span class="font-medium">Error</span>
                </div>
                <pre class="text-sm text-red-600 dark:text-red-400 overflow-x-auto">{log.error_message}</pre>
              </div>
            {/if}

            {#if log.raw_response_sample}
              <details class="mt-3">
                <summary class="text-sm text-secondary-600 dark:text-secondary-400 cursor-pointer select-none">
                  Raw response sample (first 1000 chars)
                </summary>
                <pre class="mt-2 p-3 bg-secondary-100 dark:bg-secondary-800 border border-secondary-200 dark:border-secondary-700 rounded-lg text-xs overflow-x-auto max-h-40">{log.raw_response_sample}</pre>
              </details>
            {/if}
          </div>
        </Card>
      {/each}
    </div>
  {/if}
</div>