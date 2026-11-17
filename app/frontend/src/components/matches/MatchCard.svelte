<script lang="ts">
  import { cn } from '$lib/utils';
  import ScoreBar from './ScoreBar.svelte';
  import { Building2, MapPin, Star, X, Bookmark, Check, Eye, Trash2 } from '@lucide/svelte';
  import type { MatchResult, JobPosting } from '$lib/types';

  let {
    match,
    posting,
    onView,
    onApply,
    onSave,
    onDismiss,
    compact = false,
  } = $props();

  let expanded = $state(false);

  const matchedSkills = $derived(match.matched_skills || []);
  const missingSkills = $derived(match.missing_skills || []);
  const reasons = $derived(match.reasons || []);

  function getReasonColor(matched: boolean) {
    return matched
      ? 'text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-900/20'
      : 'text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20';
  }
</script>

<article class={cn('bg-white dark:bg-secondary-800 rounded-xl border border-secondary-200 dark:border-secondary-700 overflow-hidden transition-shadow hover:shadow-md', compact && 'p-4', !compact && 'p-4')}>
  <div class="flex items-start justify-between gap-4">
    <div class="flex-1 min-w-0">
      <div class="flex items-start justify-between gap-2 mb-2">
        <h3 class="font-semibold text-secondary-900 dark:text-white truncate pr-4">
          {posting?.title || 'Untitled Position'}
        </h3>
        <ScoreBar score={match.score} size="sm" showLabel={true} label="" class="" />
      </div>

      <p class="text-secondary-600 dark:text-secondary-400 mb-2 flex items-center gap-1">
        <Building2 class="h-4 w-4" />
        {posting?.company || 'Unknown Company'}
      </p>

      <div class="flex flex-wrap items-center gap-3 mb-3 text-sm text-secondary-500">
        {#if posting?.location}
          <span class="flex items-center gap-1">
            <MapPin class="h-3.5 w-3.5" />
            {posting.location}
          </span>
        {/if}
        {#if posting?.remote_policy}
          <span class="px-2 py-0.5 bg-secondary-100 dark:bg-secondary-800 rounded text-xs">
            {posting.remote_policy}
          </span>
        {/if}
        {#if posting?.job_type}
          <span class="px-2 py-0.5 bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300 rounded text-xs">
            {posting.job_type}
          </span>
        {/if}
      </div>

      {#if matchedSkills.length > 0}
        <div class="mb-3">
          <div class="flex items-center gap-1.5 text-xs text-green-700 dark:text-green-300 mb-1">
            <Star class="h-3.5 w-3.5" />
            <span class="font-medium">Matched Skills</span>
            <span class="px-1.5 py-0.5 bg-green-100 dark:bg-green-900/30 rounded text-xs">
              {matchedSkills.length}
            </span>
          </div>
          <div class="flex flex-wrap gap-1">
            {#each matchedSkills.slice(0, 6) as skill}
              <span class="px-2 py-0.5 text-xs bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded">
                {skill}
              </span>
            {/each}
            {#if matchedSkills.length > 6}
              <span class="px-2 py-0.5 text-xs bg-secondary-100 dark:bg-secondary-800 text-secondary-600 dark:text-secondary-400 rounded">
                +{matchedSkills.length - 6}
              </span>
            {/if}
          </div>
        </div>
      {/if}

      {#if missingSkills.length > 0 && !compact}
        <div class="mb-3">
          <div class="flex items-center gap-1.5 text-xs text-red-700 dark:text-red-300 mb-1">
            <X class="h-3.5 w-3.5" />
            <span class="font-medium">Missing Skills</span>
            <span class="px-1.5 py-0.5 bg-red-100 dark:bg-red-900/30 rounded text-xs">
              {missingSkills.length}
            </span>
          </div>
          <div class="flex flex-wrap gap-1">
            {#each missingSkills.slice(0, 4) as skill}
              <span class="px-2 py-0.5 text-xs bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 rounded">
                {skill}
              </span>
            {/each}
            {#if missingSkills.length > 4}
              <span class="px-2 py-0.5 text-xs bg-secondary-100 dark:bg-secondary-800 text-secondary-600 dark:text-secondary-400 rounded">
                +{missingSkills.length - 4}
              </span>
            {/if}
          </div>
        </div>
      {/if}

      {#if reasons.length > 0 && !compact && expanded}
        <div class="mb-3 pt-3 border-t border-secondary-200 dark:border-secondary-700">
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-secondary-700 dark:text-secondary-300">Match Reasons</span>
            <button
              onclick={() => expanded = false}
              class="text-xs text-secondary-500 hover:text-secondary-700 dark:hover:text-secondary-300"
            >
              Show less
            </button>
          </div>
          <div class="space-y-1">
            {#each reasons as reason}
              <div class={cn('flex items-center gap-2 text-xs p-2 rounded-lg', getReasonColor(reason.matched))}>
                <span class="font-medium capitalize">{reason.category}</span>
                <span class="flex-1">{reason.description}</span>
                <span class="font-mono tabular-nums">
                  {Math.round(reason.weight * 100)}%
                </span>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      {#if reasons.length > 0 && !compact && !expanded}
        <button
          onclick={() => expanded = true}
          class="text-sm text-primary-600 hover:text-primary-700 dark:text-primary-400 dark:hover:text-primary-300"
        >
          Show match reasons ({reasons.length})
        </button>
      {/if}
    </div>

    <div class="flex flex-col items-end gap-2 shrink-0">
      <div class="flex items-center gap-1">
        {#if posting?.status === 'applied'}
          <span class="px-2 py-0.5 text-xs bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded-full flex items-center gap-1">
            <Check class="h-3 w-3" />
            Applied
          </span>
        {:else if posting?.status === 'saved'}
          <span class="px-2 py-0.5 text-xs bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 rounded-full flex items-center gap-1">
            <Bookmark class="h-3 w-3" />
            Saved
          </span>
        {:else if posting?.status === 'hidden'}
          <span class="px-2 py-0.5 text-xs bg-secondary-100 dark:bg-secondary-800 text-secondary-600 dark:text-secondary-400 rounded-full flex items-center gap-1">
            <Eye class="h-3 w-3" />
            Hidden
          </span>
        {/if}
      </div>

      <div class="flex items-center gap-1">
        <button
          onclick={onView}
          class="p-2 rounded-lg hover:bg-secondary-100 dark:hover:bg-secondary-800 transition-colors"
          aria-label="View details"
        >
          <Eye class="h-5 w-5 text-secondary-600 dark:text-secondary-400" />
        </button>
        {#if posting?.status !== 'applied'}
          <button
            onclick={onApply}
            class="px-3 py-1.5 text-xs font-medium bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
          >
            Apply
          </button>
        {/if}
        {#if posting?.status !== 'saved'}
          <button
            onclick={onSave}
            class="p-2 rounded-lg hover:bg-secondary-100 dark:hover:bg-secondary-800 transition-colors"
            aria-label="Save"
          >
            <Bookmark class="h-5 w-5 text-secondary-600 dark:text-secondary-400" />
          </button>
        {/if}
        <button
          onclick={onDismiss}
          class="p-2 rounded-lg hover:bg-secondary-100 dark:hover:bg-secondary-800 transition-colors"
          aria-label="Dismiss"
        >
          <Trash2 class="h-5 w-5 text-secondary-400 dark:text-secondary-500" />
        </button>
      </div>
    </div>
  </div>
</article>