<script lang="ts">
  import { onMount } from 'svelte';
  import { matchesStore, toast } from '$lib/stores';
  import { matchingApi, settingsApi } from '$lib/api';
  import { cn, getScoreColor, getScoreBg, formatRelativeTime } from '$lib/utils';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import Modal from '$components/ui/Modal.svelte';
  import MatchCard from '$components/matches/MatchCard.svelte';
  import MatchDetailModal from '$components/matches/MatchDetailModal.svelte';
  import { ChevronLeft, ChevronRight, Filter, X, Brain, Sparkles, ArrowUpDown, ArrowDownUp, Download, RefreshCw } from '@lucide/svelte';

  let selectedCV = $state<string | null>(null);
  let cvProfiles = $state<any[]>([]);
  let loading = $state(false);
  let minScore = $state(0);
  let sortBy = $state('score');
  let sortOrder = $state<'asc' | 'desc'>('desc');
  let currentPage = $state(1);
  let pageSize = $state(20);
  let showSettings = $state(false);
  let llmSettings: any = null;

  let showDetailModal = $state(false);
  let selectedMatch = $state<any>(null);

  async function loadCVs() {
    try {
      cvProfiles = await matchingApi.listCVs();
      if (cvProfiles.length > 0 && !selectedCV) {
        selectedCV = cvProfiles[0].id;
      }
    } catch (e) {
      toast('error', 'Failed to load CV profiles');
    }
  }

  async function loadMatches() {
    if (!selectedCV) return;
    loading = true;
    try {
      await matchesStore.load(selectedCV, minScore);
    } catch (e) {
      toast('error', 'Failed to load matches');
    } finally {
      loading = false;
    }
  }

  async function loadLLMSettings() {
    try {
      llmSettings = await settingsApi.getLLMSettings();
    } catch (e) {
      // LLM not configured
    }
  }

  async function rerankMatches() {
    if (!selectedCV || !llmSettings) return;
    loading = true;
    try {
      await matchesStore.rerank(llmSettings);
    } catch (e) {
      toast('error', 'Failed to rerank matches');
    } finally {
      loading = false;
    }
  }

  async function exportMatches() {
    if (!selectedCV) return;
    try {
      const matches = matchesStore.items;
      const csv = [
        ['Job Title', 'Company', 'Match Score', 'Matched Skills', 'Missing Skills', 'Location', 'Job Type', 'URL'],
        ...matches.map(m => [
          m.posting?.title || '',
          m.posting?.company || '',
          m.score.toFixed(1),
          m.matchedSkills?.join('; ') || '',
          m.missingSkills?.join('; ') || '',
          m.posting?.location || '',
          m.posting?.jobType || '',
          m.posting?.applicationUrl || '',
        ]),
      ].map(row => row.map(cell => `"${String(cell).replace(/"/g, '""')}"`).join(',')).join('\n');
      
      const blob = new Blob([csv], { type: 'text/csv' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `matches-${Date.now()}.csv`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      toast('error', 'Failed to export matches');
    }
  }

  function openDetail(match: any) {
    selectedMatch = match;
    showDetailModal = true;
  }

  onMount(async () => {
    await loadCVs();
    await loadLLMSettings();
  });

  $effect(() => {
    if (selectedCV) {
      currentPage = 1;
      loadMatches();
    }
  });

  $effect(() => {
    if (selectedCV) {
      loadMatches();
    }
  });
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-bold text-secondary-900 dark:text-white">Matches</h1>
      <p class="text-secondary-600 dark:text-secondary-400">Job matches for your CV profile</p>
    </div>
  </div>

  <Card>
    <div class="p-4 space-y-4">
      <div class="flex flex-wrap items-center gap-4">
        <div class="flex items-center gap-2">
          <label class="text-sm font-medium text-secondary-700 dark:text-secondary-300">CV Profile:</label>
          <select
            bind:value={selectedCV}
            class="px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent min-w-[200px]"
            disabled={cvProfiles.length === 0}
          >
            {#each cvProfiles as profile}
              <option value={profile.id}>{profile.name} ({profile.isDefault ? 'default' : ''})</option>
            {/each}
          </select>
        </div>

        <div class="flex items-center gap-2">
          <label class="text-sm font-medium text-secondary-700 dark:text-secondary-300">Min Score:</label>
          <input
            type="range"
            bind:value={minScore}
            min="0"
            max="100"
            step="1"
            class="w-40 h-2 bg-secondary-200 dark:bg-secondary-700 rounded-lg appearance-none accent-primary-600"
          />
          <span class="text-sm font-mono text-secondary-900 dark:text-white w-10">{minScore}%</span>
        </div>

        <div class="flex items-center gap-2">
          <label class="text-sm font-medium text-secondary-700 dark:text-secondary-300">Sort:</label>
          <select
            bind:value={sortBy}
            onchange={() => handleSort(sortBy)}
            class="px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          >
            <option value="score">Match Score</option>
            <option value="tfidf_score">TF-IDF Score</option>
            <option value="llm_score">LLM Score</option>
            <option value="created_at">Date</option>
          </select>
          <Button 
            variant="ghost" 
            size="icon" 
            onclick={() => sortOrder = sortOrder === 'asc' ? 'desc' : 'asc'}
            aria-label={sortOrder === 'asc' ? 'Sort descending' : 'Sort ascending'}
          >
            {#if sortOrder === 'asc'}
              <ArrowUpDown class="h-4 w-4" />
            {:else}
              <ArrowDownUp class="h-4 w-4" />
            {/if}
          </Button>
        </div>

        <!-- Page Size -->
        <div class="flex items-center gap-2">
          <label class="text-sm font-medium text-secondary-700 dark:text-secondary-300">Per page:</label>
          <select
            bind:value={pageSize}
            onchange={() => currentPage = 1}
            class="px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent w-24"
          >
            <option value="10">10</option>
            <option value="20">20</option>
            <option value="50">50</option>
            <option value="100">100</option>
          </select>
        </div>

        <Button variant="secondary" onclick={() => exportMatches()}>
          <Download class="h-4 w-4 mr-2" />
          Export CSV
        </Button>

        {#if llmSettings}
          <Button variant="outline" onclick={rerankMatches} disabled={loading} class="ml-auto">
            <Sparkles class="h-4 w-4 mr-2" />
            Rerank with LLM
          </Button>
        {:else}
          <Button variant="ghost" onclick={() => showSettings = true}>
            <Brain class="h-4 w-4 mr-2" />
            Configure LLM
          </Button>
        {/if}
      </div>
    </div>
  </Card>

  {#if loading}
    <div class="flex justify-center py-12">
      <svg class="animate-spin h-8 w-8 text-primary-600" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" /><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" /></svg>
    </div>
  {:else if matchesStore.items.length === 0}
    <Card>
      <div class="p-12 text-center">
        <Sparkles class="h-12 w-12 mx-auto text-secondary-300 dark:text-secondary-600 mb-4" />
        <h3 class="text-lg font-medium text-secondary-900 dark:text-white mb-2">No matches found</h3>
        <p class="text-secondary-600 dark:text-secondary-400 mb-6">Adjust your minimum score or select a different CV profile</p>
        {#if !llmSettings}
          <Button variant="outline" onclick={() => showSettings = true}>
            <Brain class="h-4 w-4 mr-2" />
            Configure LLM for better matching
          </Button>
        {/if}
      </div>
    </Card>
  {:else}
    <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {#each matchesStore.items as match}
        <MatchCard 
          match={match} 
          onClick={() => openDetail(match)}
        />
      {/each}
    </div>

    <!-- Pagination -->
    {#if matchesStore.total > pageSize}
      <div class="flex items-center justify-center gap-2">
        <Button variant="ghost" size="sm" onclick={() => currentPage > 1 && (currentPage--)} disabled={currentPage === 1}>
          <ChevronLeft class="h-4 w-4" />
        </Button>
        <span class="px-4 text-sm text-secondary-600 dark:text-secondary-400">
          Page {currentPage} of {Math.ceil(matchesStore.total / pageSize)}
        </span>
        <Button variant="ghost" size="sm" onclick={() => currentPage < Math.ceil(matchesStore.total / pageSize) && (currentPage++)} disabled={currentPage >= Math.ceil(matchesStore.total / pageSize)}>
          <ChevronRight class="h-4 w-4" />
        </Button>
      </div>
    {/if}
  {/if}
</div>

<MatchDetailModal 
  match={selectedMatch} 
  open={showDetailModal} 
  onClose={() => { showDetailModal = false; selectedMatch = null; }}
/>

<Modal open={showSettings} title="LLM Settings" on:close={() => showSettings = false}>
  <div class="p-6 space-y-4">
    <p class="text-secondary-600 dark:text-secondary-400">Configure LLM settings in the Settings page to enable AI-powered reranking.</p>
    <a href="/settings"><Button variant="primary">Open Settings</Button></a>
  </div>
</Modal>