<script lang="ts">
  import { cn, getStatusColor } from '$lib/utils';
  import Button from '$components/ui/Button.svelte';
  import Card from '$components/ui/Card.svelte';
  import Modal from '$components/ui/Modal.svelte';
  import { sourcesApi } from '$lib/api';
  import { toast } from '$lib/stores';
  import Database from '@lucide/svelte/icons/database';
  import Rss from '@lucide/svelte/icons/rss';
  import Mail from '@lucide/svelte/icons/mail';
  import Users from '@lucide/svelte/icons/users';
  import Play from '@lucide/svelte/icons/play';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Eye from '@lucide/svelte/icons/eye';
  import ToggleLeft from '@lucide/svelte/icons/toggle-left';
  import ToggleRight from '@lucide/svelte/icons/toggle-right';

  interface Source {
    id: string;
    name: string;
    source_type: string;
    config: any;
    schedule: string;
    enabled: boolean;
    status: string;
    auth_data?: any;
    last_scan_at?: string;
    last_error?: string;
    created_at: string;
    updated_at: string;
  }

  interface Props {
    source: Source;
    onScan?: (id: string) => Promise<void>;
    onToggle?: (id: string, enabled: boolean) => Promise<void>;
    onEdit?: (id: string) => void;
    onDelete?: (id: string) => void;
    onView?: (id: string) => void;
  }

  let { source, onScan, onToggle, onEdit, onDelete, onView }: Props = $props();

  let showDeleteModal = $state(false);
  let scanning = $state(false);
  let toggling = $state(false);

  function getIcon(type: string) {
    switch (type) {
      case 'url': return Database;
      case 'rss': return Rss;
      case 'gmail': return Mail;
      case 'social': return Users;
      default: return Database;
    }
  }

  let Icon = $derived.by(() => getIcon(source.source_type));

  let iconColor = $derived(source.enabled ? 'var(--color-primary-600)' : 'var(--color-secondary-500)');

  async function handleScan() {
    if (!source.enabled || scanning) return;
    scanning = true;
    try {
      if (onScan) {
        await onScan(source.id);
      } else {
        await sourcesApi.scan(source.id);
        toast('success', 'Scan started');
      }
    } catch (e) {
      toast('error', 'Failed to start scan');
    } finally {
      scanning = false;
    }
  }

  async function handleToggle() {
    if (toggling) return;
    toggling = true;
    try {
      if (onToggle) {
        await onToggle(source.id, source.enabled);
      } else {
        await sourcesApi.toggleEnabled(source.id);
        toast('success', source.enabled ? 'Source disabled' : 'Source enabled');
      }
    } catch (e) {
      toast('error', 'Failed to toggle source');
    } finally {
      toggling = false;
    }
  }

  function confirmDelete() {
    showDeleteModal = true;
  }

  async function handleDeleteConfirm() {
    try {
      if (onDelete) {
        await onDelete(source.id);
      } else {
        await sourcesApi.delete(source.id);
        toast('success', 'Source deleted');
      }
    } catch (e) {
      toast('error', 'Failed to delete source');
    } finally {
      showDeleteModal = false;
    }
  }
</script>

<Card variant="outlined" class="hover:shadow-md transition-shadow">
  <div class="p-4">
    <div class="flex items-start justify-between mb-3">
      <div class="flex items-center gap-3">
        <div class={cn('p-2 rounded-lg', source.enabled ? 'bg-primary-100 dark:bg-primary-900/30' : 'bg-secondary-100 dark:bg-secondary-800')}>
          <span style:color={iconColor}><Icon class="h-5 w-5" /></span>
        </div>
        <div>
          <h3 class="font-medium text-secondary-900 dark:text-white">{source.name}</h3>
          <span class={cn('text-xs px-2 py-0.5 rounded-full', getStatusColor(source.status))}>
            {source.status}
          </span>
        </div>
      </div>
    </div>

    <div class="space-y-2 text-sm text-secondary-600 dark:text-secondary-400 mb-4">
      <div class="flex items-center gap-2">
        <span class="w-20 font-medium">Type:</span>
        <span class="capitalize">{source.source_type}</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="w-20 font-medium">Schedule:</span>
        <span class="font-mono text-xs">{source.schedule}</span>
      </div>
      {#if source.last_scan_at}
        <div class="flex items-center gap-2">
          <span class="w-20 font-medium">Last scan:</span>
          <span>{source.last_scan_at}</span>
        </div>
      {/if}
      {#if source.last_error}
        <div class="flex items-center gap-2 text-red-600 dark:text-red-400">
          <span class="w-20 font-medium">Error:</span>
          <span class="truncate">{source.last_error}</span>
        </div>
      {/if}
    </div>

    <div class="flex items-center gap-2 pt-3 border-t border-secondary-200 dark:border-secondary-700">
      <Button
        variant={source.enabled ? 'secondary' : 'outline'}
        size="sm"
        class="flex-1"
        on:click={handleScan}
        disabled={scanning || !source.enabled}
      >
        {#if scanning}
          <svg class="animate-spin h-4 w-4 mr-2" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" /><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" /></svg>
          Scanning...
        {:else}
          <Play class="h-4 w-4 mr-2" />
          Scan Now
        {/if}
      </Button>
      {#if onToggle}
        <Button
          variant="ghost"
          size="sm"
          class={source.enabled ? 'text-green-600 hover:bg-green-50 dark:hover:bg-green-900/20' : 'text-secondary-500 hover:bg-secondary-100 dark:hover:bg-secondary-800'}
          on:click={handleToggle}
          disabled={toggling}
          title={source.enabled ? 'Disable source' : 'Enable source'}
        >
          {#if toggling}
            <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" /><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" /></svg>
          {:else if source.enabled}
            <ToggleRight class="h-4 w-4" />
          {:else}
            <ToggleLeft class="h-4 w-4" />
          {/if}
        </Button>
      {/if}
      {#if onView}
        <Button variant="ghost" size="sm" class="" on:click={() => onView!(source.id)}>
          <Eye class="h-4 w-4" />
        </Button>
      {:else if onEdit}
        <Button variant="ghost" size="sm" class="" on:click={() => onEdit!(source.id)}>
          <Pencil class="h-4 w-4" />
        </Button>
      {/if}
      {#if onDelete}
        <Button variant="ghost" size="sm" class="text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20" on:click={confirmDelete}>
          <Trash2 class="h-4 w-4" />
        </Button>
      {/if}
    </div>
  </div>
</Card>

<Modal open={showDeleteModal} title="Delete Source" on:close={() => showDeleteModal = false}>
  <p class="text-secondary-600 dark:text-secondary-400 mb-6">Are you sure you want to delete <strong>{source.name}</strong>? This action cannot be undone.</p>
  <div class="flex justify-end gap-3">
    <Button variant="secondary" class="" on:click={() => showDeleteModal = false}>Cancel</Button>
    <Button variant="destructive" class="" on:click={handleDeleteConfirm}>Delete</Button>
  </div>
</Modal>