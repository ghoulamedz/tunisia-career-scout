<script lang="ts">
  import { onMount } from 'svelte';
  import { cvStore, toast } from '$lib/stores';
  import { cvApi } from '$lib/api';
  import { cn, formatDate } from '$lib/utils';
  import { page } from '$lib/stores';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import Modal from '$components/ui/Modal.svelte';
  import {
    Plus,
    Edit,
    Trash2,
    Eye,
    Download,
    Copy,
    Check,
    FileText,
    Star,
  } from '@lucide/svelte';
  import type { CVProfile } from '$lib/types';

  let profiles = $state<CVProfile[]>([]);
  let loading = $state(true);
  let showDeleteModal = $state(false);
  let deleteTargetId = $state<string | null>(null);
  let showRenameModal = $state(false);
  let renameTargetId = $state<string | null>(null);
  let renameValue = $state('');

  async function loadProfiles() {
    loading = true;
    try {
      profiles = await cvApi.list();
    } catch (e) {
      console.error('Failed to load CVs:', e);
      toast('error', 'Failed to load CV profiles');
    } finally {
      loading = false;
    }
  }

  async function createNewCV() {
    page.navigate('/cv/editor');
  }

  async function editCV(profile: CVProfile) {
    page.navigate(`/cv/editor?id=${profile.id}`);
  }

  async function previewCV(profile: CVProfile) {
    page.navigate(`/cv/preview?id=${profile.id}`);
  }

  async function downloadCV(profile: CVProfile) {
    try {
      const pdf = await cvApi.generatePDF(profile.id);
      const blob = new Blob([pdf], { type: 'application/pdf' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${profile.name}.pdf`;
      a.click();
      URL.revokeObjectURL(url);
      toast('success', 'PDF downloaded');
    } catch (e) {
      console.error('Failed to generate PDF:', e);
      toast('error', 'Failed to generate PDF');
    }
  }

  async function duplicateCV(profile: CVProfile) {
    try {
      const newProfile = await cvApi.save({
        name: `${profile.name} (Copy)`,
        data: profile.data,
        is_default: false,
      });
      toast('success', 'CV duplicated');
      await loadProfiles();
    } catch (e) {
      console.error('Failed to duplicate CV:', e);
      toast('error', 'Failed to duplicate CV');
    }
  }

  async function setDefaultCV(id: string) {
    try {
      await cvStore.setDefault(id);
      toast('success', 'Set as default CV');
      await loadProfiles();
    } catch (e) {
      console.error('Failed to set default:', e);
      toast('error', 'Failed to set default CV');
    }
  }

  function confirmDelete(id: string) {
    deleteTargetId = id;
    showDeleteModal = true;
  }

  async function deleteCV() {
    if (!deleteTargetId) return;
    try {
      await cvApi.delete(deleteTargetId);
      toast('success', 'CV deleted');
      await loadProfiles();
    } catch (e) {
      console.error('Failed to delete CV:', e);
      toast('error', 'Failed to delete CV');
    } finally {
      showDeleteModal = false;
      deleteTargetId = null;
    }
  }

  function startRename(id: string, currentName: string) {
    renameTargetId = id;
    renameValue = currentName;
    showRenameModal = true;
  }

  async function renameCV() {
    if (!renameTargetId || !renameValue.trim()) return;
    try {
      await cvApi.update(renameTargetId, { name: renameValue.trim() });
      toast('success', 'CV renamed');
      await loadProfiles();
    } catch (e) {
      console.error('Failed to rename CV:', e);
      toast('error', 'Failed to rename CV');
    } finally {
      showRenameModal = false;
      renameTargetId = null;
      renameValue = '';
    }
  }

  onMount(loadProfiles);
</script>

<div class="space-y-6">
  <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
    <div>
      <h1 class="text-2xl font-bold text-secondary-900 dark:text-white">CV Profiles</h1>
      <p class="text-secondary-600 dark:text-secondary-400">Manage your CV profiles and templates</p>
    </div>
    <Button on:click={createNewCV}>
      <Plus class="h-4 w-4 mr-2" />
      Create New CV
    </Button>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <Loader2 class="h-8 w-8 animate-spin text-primary-500" />
      <span class="sr-only">Loading CV profiles...</span>
    </div>
  {:else if profiles.length === 0}
    <Card class="text-center py-12">
      <FileText class="h-16 w-16 text-secondary-300 dark:text-secondary-600 mx-auto mb-4" />
      <h3 class="text-lg font-medium text-secondary-900 dark:text-white mb-2">No CV profiles yet</h3>
      <p class="text-secondary-600 dark:text-secondary-400 mb-6">Create your first CV profile to get started</p>
      <Button on:click={createNewCV} class="w-full sm:w-auto">
        <Plus class="h-4 w-4 mr-2" />
        Create Your First CV
      </Button>
    </Card>
  {:else}
    <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {#each profiles as profile}
        <Card class="relative flex flex-col">
          {#if profile.isDefault}
            <div class="absolute top-3 right-3 flex items-center gap-1 text-amber-500 bg-amber-50 dark:bg-amber-900/30 px-2 py-1 rounded-full text-xs font-medium">
              <Star class="h-3 w-3 fill-current" />
              Default
            </div>
          {/if}
          
          <div class="p-6 flex-1 flex flex-col">
            <div class="flex items-start justify-between mb-4">
              <div class="flex-1 min-w-0">
                <h3 class="font-semibold text-secondary-900 dark:text-white truncate pr-2">{profile.name}</h3>
                <p class="text-sm text-secondary-500 dark:text-secondary-400 mt-1">
                  {profile.template} • {profile.theme}
                </p>
              </div>
            </div>

            <div class="flex items-center gap-2 text-sm text-secondary-500 dark:text-secondary-400 mb-4">
              <span>Created: {formatDate(profile.createdAt)}</span>
              <span>•</span>
              <span>Updated: {formatDate(profile.updatedAt)}</span>
            </div>

            <div class="flex flex-wrap gap-2 mt-auto">
              <Button
                variant="primary"
                size="sm"
                class="flex-1 min-w-[80px]"
                on:click={() => editCV(profile)}
              >
                <Edit class="h-3.5 w-3.5 mr-1.5" />
                Edit
              </Button>
              <Button
                variant="secondary"
                size="sm"
                class="flex-1 min-w-[80px]"
                on:click={() => previewCV(profile)}
              >
                <Eye class="h-3.5 w-3.5 mr-1.5" />
                Preview
              </Button>
              {#if !profile.isDefault}
                <Button
                  variant="ghost"
                  size="sm"
                  on:click={() => setDefaultCV(profile.id)}
                  title="Set as default"
                  class="text-amber-600 hover:bg-amber-50 dark:hover:bg-amber-900/30"
                >
                  <Star class="h-3.5 w-3.5" />
                </Button>
              {/if}
              <Button
                variant="ghost"
                size="sm"
                on:click={() => downloadCV(profile)}
                title="Download PDF"
              >
                <Download class="h-3.5 w-3.5" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                on:click={() => duplicateCV(profile)}
                title="Duplicate"
              >
                <Copy class="h-3.5 w-3.5" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="text-red-600 hover:bg-red-50 dark:hover:bg-red-900/30"
                on:click={() => confirmDelete(profile.id)}
                title="Delete"
              >
                <Trash2 class="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>
        </Card>
      {/each}
    </div>
  {/if}
</div>

{#if showDeleteModal}
  <Modal open={true} title="Delete CV" on:close={() => { showDeleteModal = false; deleteTargetId = null; }}>
    <div class="p-6 space-y-4">
      <p class="text-secondary-600 dark:text-secondary-400">Are you sure you want to delete this CV profile? This action cannot be undone.</p>
      <div class="flex justify-end gap-3">
        <Button variant="secondary" on:click={() => { showDeleteModal = false; deleteTargetId = null; }}>Cancel</Button>
        <Button variant="destructive" on:click={deleteCV}>Delete</Button>
      </div>
    </div>
  </Modal>
{/if}

{#if showRenameModal}
  <Modal open={true} title="Rename CV" on:close={() => { showRenameModal = false; renameTargetId = null; renameValue = ''; }}>
    <div class="p-6 space-y-4">
      <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300">New name</label>
      <input
        bind:value={renameValue}
        type="text"
        class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
        on:keydown={(e) => e.key === 'Enter' && renameCV()}
      />
      <div class="flex justify-end gap-3">
        <Button variant="secondary" on:click={() => { showRenameModal = false; renameTargetId = null; renameValue = ''; }}>Cancel</Button>
        <Button variant="primary" on:click={renameCV} disabled={!renameValue.trim()}>Save</Button>
      </div>
    </div>
  </Modal>
{/if}