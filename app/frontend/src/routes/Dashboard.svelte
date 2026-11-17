<script lang="ts">
  import { onMount } from 'svelte';
  import { sourcesStore, postingsStore, cvStore, matchesStore, toasts, toast } from '$lib/stores';
  import { sourcesApi, postingsApi, cvApi, matchingApi, settingsApi } from '$lib/api';
  import { cn, formatRelativeTime, getScoreColor, getScoreBg } from '$lib/utils';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import Briefcase from '@lucide/svelte/icons/briefcase';
  import Database from '@lucide/svelte/icons/database';
  import FileText from '@lucide/svelte/icons/file-text';
  import Target from '@lucide/svelte/icons/target';
  import Plus from '@lucide/svelte/icons/plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';

  let stats = $state({
    sources: 0,
    postings: 0,
    cvProfiles: 0,
    matches: 0,
  });

  let recentPostings = $state([] as any[]);
  let topMatches = $state([] as any[]);
  let loading = $state(true);

  async function loadDashboard() {
    loading = true;
    try {
      const [sources, postings, cvs] = await Promise.all([
        sourcesApi.list(true),
        postingsApi.list({ status: 'active', limit: 5 }),
        cvApi.list(),
      ]);

      stats.sources = sources.length;
      stats.postings = postings.length;
      stats.cvProfiles = cvs.length;

      recentPostings = postings;

      // Get matches for default CV
      const defaultCv = cvs.find(c => c.is_default) || cvs[0];
      if (defaultCv) {
        const matches = await matchingApi.list(defaultCv.id, 50, 5);
        topMatches = matches;
        stats.matches = matches.length;
      }
    } catch (e) {
      toast('error', 'Failed to load dashboard');
    } finally {
      loading = false;
    }
  }

  async function scanAllSources() {
    try {
      const sources = await sourcesApi.list(true);
      for (const source of sources) {
        await sourcesApi.scan(source.id);
      }
      toast('success', 'Scanning started for all sources');
      loadDashboard();
    } catch (e) {
      toast('error', 'Failed to start scans');
    }
  }

  onMount(loadDashboard);
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-bold text-secondary-900 dark:text-white">Dashboard</h1>
      <p class="text-secondary-600 dark:text-secondary-400">Overview of your job search</p>
    </div>
    <Button variant="primary" on:click={scanAllSources} disabled={loading}>
      <RefreshCw class="h-4 w-4" />
      Scan All Sources
    </Button>
  </div>

  <!-- Stats Grid -->
  <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
    <Card>
      <div class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-secondary-600 dark:text-secondary-400">Active Sources</p>
            <p class="text-3xl font-bold text-secondary-900 dark:text-white">{stats.sources}</p>
          </div>
          <div class="p-3 bg-blue-100 dark:bg-blue-900/30 rounded-xl text-blue-600 dark:text-blue-400">
            <Database class="h-6 w-6" />
          </div>
        </div>
      </div>
    </Card>

    <Card>
      <div class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-secondary-600 dark:text-secondary-400">Job Postings</p>
            <p class="text-3xl font-bold text-secondary-900 dark:text-white">{stats.postings}</p>
          </div>
          <div class="p-3 bg-green-100 dark:bg-green-900/30 rounded-xl text-green-600 dark:text-green-400">
            <Briefcase class="h-6 w-6" />
          </div>
        </div>
      </div>
    </Card>

    <Card>
      <div class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-secondary-600 dark:text-secondary-400">CV Profiles</p>
            <p class="text-3xl font-bold text-secondary-900 dark:text-white">{stats.cvProfiles}</p>
          </div>
          <div class="p-3 bg-purple-100 dark:bg-purple-900/30 rounded-xl text-purple-600 dark:text-purple-400">
            <FileText class="h-6 w-6" />
          </div>
        </div>
      </div>
    </Card>

    <Card>
      <div class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-secondary-600 dark:text-secondary-400">High Matches</p>
            <p class="text-3xl font-bold text-secondary-900 dark:text-white">{stats.matches}</p>
          </div>
          <div class="p-3 bg-orange-100 dark:bg-orange-900/30 rounded-xl text-orange-600 dark:text-orange-400">
            <Target class="h-6 w-6" />
          </div>
        </div>
      </div>
    </Card>
  </div>

  <!-- Recent Postings & Top Matches -->
  <div class="grid gap-6 lg:grid-cols-2">
    <Card>
      <div class="p-4 border-b border-secondary-200 dark:border-secondary-700 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Recent Job Postings</h2>
        <a href="/postings" class="text-sm text-primary-600 hover:underline">View all</a>
      </div>
      <div class="divide-y divide-secondary-200 dark:divide-secondary-700">
        {#if loading}
          <div class="p-8 text-center text-secondary-500">Loading...</div>
        {:else if recentPostings.length === 0}
          <div class="p-8 text-center text-secondary-500">
            No postings yet. <a href="/sources/new" class="text-primary-600 underline">Add a source</a> to start scraping.
          </div>
        {:else}
          {#each recentPostings as posting}
            <a href="/postings/{posting.id}" class="block p-4 hover:bg-secondary-50 dark:hover:bg-secondary-800/50 transition-colors">
              <div class="flex items-start justify-between gap-4">
                <div class="flex-1 min-w-0">
                  <h3 class="font-medium text-secondary-900 dark:text-white truncate">{posting.title}</h3>
                  <p class="text-sm text-secondary-600 dark:text-secondary-400">{posting.company}</p>
                  <div class="flex items-center gap-2 mt-1 text-xs text-secondary-500">
                    <span>{formatRelativeTime(posting.scraped_at)}</span>
                    {#if posting.location}
                      <span>·</span>
                      <span>{posting.location}</span>
                    {/if}
                  </div>
                </div>
              </div>
            </a>
          {/each}
        {/if}
      </div>
    </Card>

    <Card>
      <div class="p-4 border-b border-secondary-200 dark:border-secondary-700 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Top Matches</h2>
        <a href="/matches" class="text-sm text-primary-600 hover:underline">View all</a>
      </div>
      <div class="divide-y divide-secondary-200 dark:divide-secondary-700">
        {#if topMatches.length === 0}
          <div class="p-8 text-center text-secondary-500">
            No matches yet. <a href="/cv" class="text-primary-600 underline">Create a CV</a> and run matching.
          </div>
        {:else}
          {#each topMatches as match}
            <div class="p-4">
              <div class="flex items-start justify-between gap-4">
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-1">
                    <h3 class="font-medium text-secondary-900 dark:text-white truncate">{match.posting?.title || 'Loading...'}</h3>
                    <span class={cn('px-2 py-0.5 text-xs font-medium rounded-full', getScoreBg(match.score), getScoreColor(match.score))}>
                      {Math.round(match.score)}%
                    </span>
                  </div>
                  <p class="text-sm text-secondary-600 dark:text-secondary-400">{match.posting?.company || ''}</p>
                  <div class="flex flex-wrap gap-1 mt-2">
                    {#each match.matched_skills.slice(0, 3) as skill}
                      <span class="px-2 py-0.5 text-xs bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 rounded">{skill}</span>
                    {/each}
                    {#if match.matched_skills.length > 3}
                      <span class="px-2 py-0.5 text-xs bg-secondary-100 text-secondary-600 dark:bg-secondary-800 dark:text-secondary-400 rounded">+{match.matched_skills.length - 3}</span>
                    {/if}
                  </div>
                </div>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </Card>
  </div>

  <!-- Quick Actions -->
  <Card>
    <div class="p-6">
      <h2 class="text-lg font-semibold mb-4">Quick Actions</h2>
      <div class="grid gap-4 sm:grid-cols-3">
        <a href="/sources/new" class="p-4 border border-secondary-200 dark:border-secondary-700 rounded-xl hover:bg-secondary-50 dark:hover:bg-secondary-800/50 transition-colors group">
          <div class="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg text-primary-600 dark:text-primary-400 mb-3">
            <Plus class="h-6 w-6" />
          </div>
          <h3 class="font-medium text-secondary-900 dark:text-white group-hover:text-primary-600 transition-colors">Add Source</h3>
          <p class="text-sm text-secondary-600 dark:text-secondary-400 mt-1">Configure a new job source (URL, RSS, Gmail, Social)</p>
        </a>

        <a href="/cv" class="p-4 border border-secondary-200 dark:border-secondary-700 rounded-xl hover:bg-secondary-50 dark:hover:bg-secondary-800/50 transition-colors group">
          <div class="p-2 bg-purple-100 dark:bg-purple-900/30 rounded-lg text-purple-600 dark:text-purple-400 mb-3">
            <FileText class="h-6 w-6" />
          </div>
          <h3 class="font-medium text-secondary-900 dark:text-white group-hover:text-primary-600 transition-colors">Build CV</h3>
          <p class="text-sm text-secondary-600 dark:text-secondary-400 mt-1">Create or update your professional CV profile</p>
        </a>

        <a href="/settings" class="p-4 border border-secondary-200 dark:border-secondary-700 rounded-xl hover:bg-secondary-50 dark:hover:bg-secondary-800/50 transition-colors group">
          <div class="p-2 bg-secondary-100 dark:bg-secondary-800 rounded-lg text-secondary-600 dark:text-secondary-400 mb-3">
            <svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>
          </div>
          <h3 class="font-medium text-secondary-900 dark:text-white group-hover:text-primary-600 transition-colors">Settings</h3>
          <p class="text-sm text-secondary-600 dark:text-secondary-400 mt-1">Configure LLM, notifications, sync, and more</p>
        </a>
      </div>
    </div>
  </Card>
</div>