<script lang="ts">
  import { cn, formatDate } from '$lib/utils';
  import Modal from '$components/ui/Modal.svelte';
  import Button from '$components/ui/Button.svelte';
  import ScoreBar from './ScoreBar.svelte';
  import { 
    Briefcase, MapPin, Globe, DollarSign, Mail, ExternalLink, 
    Clock, Star, X, Check, Bookmark, Eye, Trash2,
    Code, GraduationCap, Languages, Phone
  } from '@lucide/svelte';
  import type { MatchResult, JobPosting, MatchReason, ApplicationStep } from '$lib/types';

  let {
    open = false,
    match,
    posting,
    onClose,
    onApply,
    onSave,
    onDismiss,
  } = $props();

  let activeTab = $state<'overview' | 'skills' | 'reasons' | 'application'>('overview');

  const matchedSkills = $derived(match?.matched_skills || []);
  const missingSkills = $derived(match?.missing_skills || []);
  const reasons = $derived(match?.reasons || []);

  function getReasonColor(matched: boolean) {
    return matched
      ? 'text-green-700 dark:text-green-300 bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800'
      : 'text-red-700 dark:text-red-300 bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800';
  }

  const tabs = [
    { id: 'overview', label: 'Overview', icon: Eye },
    { id: 'skills', label: 'Skills', icon: Code },
    { id: 'reasons', label: 'Reasons', icon: Star },
    { id: 'application', label: 'Apply', icon: ExternalLink },
  ] as const;
</script>

<Modal open={open} title="" size="xl" onclose={onClose}>
  <div class="flex items-center justify-between mb-4">
    <div>
      <h2 id="modal-title" class="text-xl font-bold text-secondary-900 dark:text-white">
        {posting?.title || 'Job Details'}
      </h2>
      <p class="text-secondary-600 dark:text-secondary-400">{posting?.company || 'Unknown Company'}</p>
    </div>
    <ScoreBar score={match?.score || 0} size="lg" label="Overall Match" class="" />
  </div>

  <!-- Tab Navigation -->
  <div class="flex border-b border-secondary-200 dark:border-secondary-700 mb-4 -mx-4 px-4" role="tablist">
    {#each tabs as tab}
      <button
        role="tab"
        aria-selected={activeTab === tab.id}
        aria-controls={`panel-${tab.id}`}
        onclick={() => activeTab = tab.id}
        class={cn(
          'flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 transition-colors',
          activeTab === tab.id
            ? 'border-primary-500 text-primary-600 dark:text-primary-400'
            : 'border-transparent text-secondary-500 hover:text-secondary-700 dark:text-secondary-400 dark:hover:text-secondary-200'
        )}
      >
        <tab.icon class="h-4 w-4" />
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Overview Tab -->
  <div role="tabpanel" id="panel-overview" class={activeTab === 'overview' ? '' : 'hidden'}>
    <div class="space-y-6">
      <!-- Meta Info -->
      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {#if posting?.location}
          <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
            <MapPin class="h-5 w-5 text-secondary-500 mb-1" />
            <p class="text-xs text-secondary-500 dark:text-secondary-400">Location</p>
            <p class="font-medium text-secondary-900 dark:text-white">{posting.location}</p>
          </div>
        {/if}
        {#if posting?.remote_policy}
          <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
            <Globe class="h-5 w-5 text-secondary-500 mb-1" />
            <p class="text-xs text-secondary-500 dark:text-secondary-400">Remote Policy</p>
            <p class="font-medium text-secondary-900 dark:text-white">{posting.remote_policy}</p>
          </div>
        {/if}
        {#if posting?.job_type}
          <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
            <Briefcase class="h-5 w-5 text-secondary-500 mb-1" />
            <p class="text-xs text-secondary-500 dark:text-secondary-400">Job Type</p>
            <p class="font-medium text-secondary-900 dark:text-white">{posting.job_type}</p>
          </div>
        {/if}
        {#if posting?.salary_range}
          <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
            <DollarSign class="h-5 w-5 text-secondary-500 mb-1" />
            <p class="text-xs text-secondary-500 dark:text-secondary-400">Salary Range</p>
            <p class="font-medium text-secondary-900 dark:text-white">{posting.salary_range}</p>
          </div>
        {/if}
      </div>

      <!-- Summary -->
      {#if posting?.summary}
        <div class="prose dark:prose-invert max-w-none">
          <h3 class="font-semibold mb-2">Summary</h3>
          <p class="whitespace-pre-wrap">{posting.summary}</p>
        </div>
      {/if}

      <!-- Description -->
      <div class="prose dark:prose-invert max-w-none">
        <h3 class="font-semibold mb-2">Full Description</h3>
        <div class="whitespace-pre-wrap">{posting?.description || 'No description available'}</div>
      </div>

      <!-- Requirements -->
      <div class="grid gap-4 md:grid-cols-2">
        {#if posting?.experience_required}
          <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
            <h4 class="font-medium text-secondary-900 dark:text-white mb-2 flex items-center gap-2">
              <Briefcase class="h-5 w-5 text-secondary-500" />
              Experience Required
            </h4>
            <p class="text-secondary-600 dark:text-secondary-400">{posting.experience_required}</p>
          </div>
        {/if}
        {#if posting?.education_required}
          <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
            <h4 class="font-medium text-secondary-900 dark:text-white mb-2 flex items-center gap-2">
              <GraduationCap class="h-5 w-5 text-secondary-500" />
              Education Required
            </h4>
            <p class="text-secondary-600 dark:text-secondary-400">{posting.education_required}</p>
          </div>
        {/if}
        {#if posting?.languages.length > 0}
          <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
            <h4 class="font-medium text-secondary-900 dark:text-white mb-2 flex items-center gap-2">
              <Languages class="h-5 w-5 text-secondary-500" />
              Languages
            </h4>
            <p class="text-secondary-600 dark:text-secondary-400">{posting.languages.join(', ')}</p>
          </div>
        {/if}
        {#if posting?.deadline}
          <div class="p-4 bg-red-50 dark:bg-red-900/20 rounded-lg border border-red-200 dark:border-red-800">
            <h4 class="font-medium text-red-700 dark:text-red-300 mb-2 flex items-center gap-2">
              <Clock class="h-5 w-5" />
              Application Deadline
            </h4>
            <p class="text-red-600 dark:text-red-400 font-medium">{formatDate(posting.deadline)}</p>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Skills Tab -->
  <div role="tabpanel" id="panel-skills" class={activeTab === 'skills' ? '' : 'hidden'}>
    <div class="space-y-6">
      {#if matchedSkills.length > 0}
        <div>
          <h3 class="font-semibold text-green-700 dark:text-green-300 mb-3 flex items-center gap-2">
            <Star class="h-5 w-5" />
            Matched Skills ({matchedSkills.length})
          </h3>
          <div class="flex flex-wrap gap-2">
            {#each matchedSkills as skill}
              <span class="px-3 py-1.5 bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded-lg text-sm border border-green-200 dark:border-green-800">
                {skill}
              </span>
            {/each}
          </div>
        </div>
      {:else}
        <div class="text-center py-8 text-secondary-500">
          <Star class="h-12 w-12 mx-auto mb-3 opacity-30" />
          <p>No matched skills found</p>
        </div>
      {/if}

      {#if missingSkills.length > 0}
        <div class="pt-6 border-t border-secondary-200 dark:border-secondary-700">
          <h3 class="font-semibold text-red-700 dark:text-red-300 mb-3 flex items-center gap-2">
            <X class="h-5 w-5" />
            Missing Skills ({missingSkills.length})
          </h3>
          <div class="flex flex-wrap gap-2">
            {#each missingSkills as skill}
              <span class="px-3 py-1.5 bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 rounded-lg text-sm border border-red-200 dark:border-red-800">
                {skill}
              </span>
            {/each}
          </div>
        </div>
      {:else}
        <div class="text-center py-8 text-secondary-500">
          <X class="h-12 w-12 mx-auto mb-3 opacity-30" />
          <p>No missing skills identified</p>
        </div>
      {/if}

      {#if posting?.skills.length > 0}
        <div class="pt-6 border-t border-secondary-200 dark:border-secondary-700">
          <h3 class="font-semibold text-secondary-900 dark:text-white mb-3 flex items-center gap-2">
            <Code class="h-5 w-5 text-secondary-500" />
            All Required Skills ({posting.skills.length})
          </h3>
          <div class="flex flex-wrap gap-2">
            {#each posting.skills as skill}
              <span class={cn(
                'px-3 py-1.5 rounded-lg text-sm border transition-colors',
                matchedSkills.includes(skill)
                  ? 'bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300 border-green-200 dark:border-green-800'
                  : missingSkills.includes(skill)
                    ? 'bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300 border-red-200 dark:border-red-800'
                    : 'bg-secondary-100 dark:bg-secondary-800 text-secondary-700 dark:text-secondary-300 border-secondary-200 dark:border-secondary-700'
              )}>
                {skill}
                {#if matchedSkills.includes(skill)}
                  <Star class="h-3.5 w-3.5 ml-1 inline" />
                {:else if missingSkills.includes(skill)}
                  <X class="h-3.5 w-3.5 ml-1 inline" />
                {/if}
              </span>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>

  <!-- Reasons Tab -->
  <div role="tabpanel" id="panel-reasons" class={activeTab === 'reasons' ? '' : 'hidden'}>
    <div class="space-y-4">
      {#if reasons.length > 0}
        {#each reasons as reason}
          <div class={cn('p-4 rounded-lg border flex items-start gap-3', getReasonColor(reason.matched))}>
            {#if reason.matched}
              <Star class="h-5 w-5 flex-shrink-0 mt-0.5 text-green-600 dark:text-green-400" />
            {:else}
              <X class="h-5 w-5 flex-shrink-0 mt-0.5 text-red-600 dark:text-red-400" />
            {/if}
            <div class="flex-1">
              <div class="flex items-center justify-between mb-1">
                <span class="font-medium capitalize">{reason.category}</span>
                <span class="text-sm font-mono tabular-nums text-secondary-500">
                  {Math.round(reason.weight * 100)}% weight
                </span>
              </div>
              <p class="text-sm">{reason.description}</p>
            </div>
          </div>
        {/each}
      {:else}
        <div class="text-center py-12 text-secondary-500">
          <Star class="h-12 w-12 mx-auto mb-3 opacity-30" />
          <p>No match reasons available</p>
          <p class="text-sm mt-1">Run LLM re-ranking to get detailed reasoning</p>
        </div>
      {/if}

      {#if match?.tfidf_score !== undefined || match?.llm_score !== undefined}
        <div class="pt-4 border-t border-secondary-200 dark:border-secondary-700">
          <h4 class="font-medium text-secondary-900 dark:text-white mb-3">Score Breakdown</h4>
          <div class="space-y-3">
            {#if match?.tfidf_score !== undefined}
              <div>
                <div class="flex justify-between text-sm mb-1">
                  <span class="text-secondary-600 dark:text-secondary-400">TF-IDF Score</span>
                  <span class="font-medium text-secondary-900 dark:text-white">{Math.round(match.tfidf_score)}%</span>
                </div>
                <div class="h-2 bg-secondary-200 dark:bg-secondary-700 rounded-full overflow-hidden">
                  <div class="h-full bg-blue-500 rounded-full" style="width: {Math.min(100, Math.max(0, match.tfidf_score))}%"></div>
                </div>
              </div>
            {/if}
            {#if match?.llm_score !== undefined}
              <div>
                <div class="flex justify-between text-sm mb-1">
                  <span class="text-secondary-600 dark:text-secondary-400">LLM Re-rank Score</span>
                  <span class="font-medium text-secondary-900 dark:text-white">{Math.round(match.llm_score)}%</span>
                </div>
                <div class="h-2 bg-secondary-200 dark:bg-secondary-700 rounded-full overflow-hidden">
                  <div class="h-full bg-purple-500 rounded-full" style="width: {Math.min(100, Math.max(0, match.llm_score))}%"></div>
                </div>
              </div>
            {/if}
            <div>
              <div class="flex justify-between text-sm mb-1">
                <span class="text-secondary-600 dark:text-secondary-400">Final Score</span>
                <span class="font-bold text-secondary-900 dark:text-white">{Math.round(match?.score || 0)}%</span>
              </div>
              <div class="h-2 bg-secondary-200 dark:bg-secondary-700 rounded-full overflow-hidden">
                <div class={cn('h-full rounded-full', 
                  match?.score >= 80 && 'bg-green-500',
                  match?.score >= 60 && match?.score < 80 && 'bg-yellow-500',
                  match?.score >= 40 && match?.score < 60 && 'bg-orange-500',
                  match?.score < 40 && 'bg-red-500'
                )} style="width: {Math.min(100, Math.max(0, match?.score || 0))}%"></div>
              </div>
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <!-- Application Tab -->
  <div role="tabpanel" id="panel-application" class={activeTab === 'application' ? '' : 'hidden'}>
    <div class="space-y-6">
      <!-- Application Steps -->
      {#if posting?.application_steps.length > 0}
        <div>
          <h3 class="font-semibold text-secondary-900 dark:text-white mb-4">Application Steps</h3>
          <div class="space-y-4">
            {#each posting.application_steps as step}
              <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg border border-secondary-200 dark:border-secondary-700">
                <div class="flex items-start gap-4">
                  <div class="flex-shrink-0 w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center text-primary-700 dark:text-primary-300 font-bold">
                    {step.order}
                  </div>
                  <div class="flex-1">
                    <p class="font-medium text-secondary-900 dark:text-white mb-2">{step.description}</p>
                    <div class="flex flex-wrap items-center gap-3 text-sm text-secondary-600 dark:text-secondary-400">
                      <span class={cn('px-2 py-0.5 rounded text-xs font-medium',
                        step.action_type === 'link' && 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300',
                        step.action_type === 'email' && 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300',
                        step.action_type === 'form' && 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300',
                        step.action_type === 'upload' && 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-300',
                        'bg-secondary-100 text-secondary-700 dark:bg-secondary-800 dark:text-secondary-300'
                      )}>
                        {step.action_type}
                      </span>
                      <a href={step.target} target="_blank" class="text-primary-600 hover:underline flex items-center gap-1">
                        <ExternalLink class="h-3.5 w-3.5" />
                        {step.target}
                      </a>
                      {#if step.deadline}
                        <span class="text-red-600 flex items-center gap-1">
                          <Clock class="h-3.5 w-3.5" />
                          Due: {formatDate(step.deadline)}
                        </span>
                      {/if}
                      {#if step.required}
                        <span class="px-2 py-0.5 bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 rounded text-xs">Required</span>
                      {/if}
                      {#if step.completed}
                        <span class="px-2 py-0.5 bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded text-xs">Completed</span>
                      {/if}
                    </div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="text-center py-8 text-secondary-500">
          <ExternalLink class="h-12 w-12 mx-auto mb-3 opacity-30" />
          <p>No structured application steps available</p>
        </div>
      {/if}

      <!-- Quick Actions -->
      <div class="pt-4 border-t border-secondary-200 dark:border-secondary-700">
        <h3 class="font-semibold text-secondary-900 dark:text-white mb-3">Quick Actions</h3>
        <div class="flex flex-wrap gap-3">
          {#if posting?.application_url}
            <a href={posting.application_url} target="_blank">
              <Button variant="primary" class="w-full sm:w-auto">
                <ExternalLink class="h-4 w-4 mr-2" />
                View Original Posting
              </Button>
            </a>
          {/if}
          {#if posting?.application_email}
            <a href={`mailto:${posting.application_email}`}>
              <Button variant="secondary" class="w-full sm:w-auto">
                <Mail class="h-4 w-4 mr-2" />
                Apply via Email
              </Button>
            </a>
          {/if}
          {#if posting?.contact_phone}
            <a href={`tel:${posting.contact_phone}`}>
              <Button variant="outline" class="w-full sm:w-auto">
                <Phone class="h-4 w-4 mr-2" />
                Call Contact
              </Button>
            </a>
          {/if}
        </div>
      </div>

      {#if posting?.contact_person}
        <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
          <h4 class="font-medium text-secondary-900 dark:text-white mb-2">Contact Person</h4>
          <p class="text-secondary-600 dark:text-secondary-400">{posting.contact_person}</p>
          {#if posting?.contact_phone}
            <p class="text-secondary-600 dark:text-secondary-400">{posting.contact_phone}</p>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- Footer Actions -->
  <div class="mt-6 flex items-center justify-end gap-3 border-t border-secondary-200 dark:border-secondary-700 pt-4 -mx-4 px-4">
    <Button variant="ghost" onclick={onDismiss} class="">
      <Trash2 class="h-4 w-4 mr-2" />
      Dismiss
    </Button>
    {#if posting?.status !== 'saved'}
      <Button variant="secondary" onclick={onSave} class="">
        <Bookmark class="h-4 w-4 mr-2" />
        Save
      </Button>
    {/if}
    {#if posting?.status !== 'applied'}
      <Button variant="primary" onclick={onApply} class="">
        <Check class="h-4 w-4 mr-2" />
        Mark Applied
      </Button>
    {/if}
    <Button variant="ghost" onclick={onClose} class="">Close</Button>
  </div>
</Modal>
