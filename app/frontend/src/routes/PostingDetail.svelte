<script lang="ts">
  import { onMount } from 'svelte';
  import { page, postingsStore, toast } from '$lib/stores';
  import { postingsApi } from '$lib/api';
  import { cn, formatDate, getStatusColor } from '$lib/utils';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import Modal from '$components/ui/Modal.svelte';
  import { ChevronLeft, Briefcase, MapPin, Clock, Globe, DollarSign, Mail, ExternalLink, RefreshCw, FileText, Trash2, Check, X } from '@lucide/svelte';

  let posting = $state(null as any);
  let loading = $state(true);
  let showReExtractModal = $state(false);
  let reExtractLoading = $state(false);
  let llmSettings = $state(null as any);

  async function loadPosting() {
    loading = true;
    try {
      posting = await postingsApi.get(page.params.id);
    } catch (e) {
      toast('error', 'Failed to load posting');
    } finally {
      loading = false;
    }
  }

  async function updateStatus(status: string) {
    if (!posting) return;
    try {
      await postingsApi.updateStatus(posting.id, status);
      posting.status = status;
      toast('success', `Status updated to ${status}`);
    } catch (e) {
      toast('error', 'Failed to update status');
    }
  }

  async function handleReExtract() {
    if (!posting || !llmSettings) return;
    reExtractLoading = true;
    try {
      const updated = await postingsApi.reExtract(posting.id, llmSettings);
      posting = updated;
      toast('success', 'Re-extraction complete');
      showReExtractModal = false;
    } catch (e) {
      toast('error', 'Re-extraction failed');
    } finally {
      reExtractLoading = false;
    }
  }

  onMount(loadPosting);
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <a href="/postings" class="flex items-center gap-2 text-secondary-600 hover:text-secondary-900 dark:text-secondary-400 dark:hover:text-white">
      <ChevronLeft class="h-5 w-5" />
      Back
    </a>
    <div class="flex items-center gap-2">
      {#if posting}
        <Button variant="secondary" on:click={() => updateStatus('saved')} disabled={posting.status === 'saved'}>
          <Check class="h-4 w-4 mr-2" />
          Save
        </Button>
        <Button variant="secondary" on:click={() => updateStatus('applied')} disabled={posting.status === 'applied'}>
          <Check class="h-4 w-4 mr-2" />
          Applied
        </Button>
        <Button variant="secondary" on:click={() => showReExtractModal = true}>
          <RefreshCw class="h-4 w-4 mr-2" />
          Re-extract (LLM)
        </Button>
      {/if}
    </div>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <svg class="animate-spin h-8 w-8 text-primary-600" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" /><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" /></svg>
    </div>
  {:else if !posting}
    <Card>
      <div class="p-12 text-center">
        <FileText class="h-12 w-12 mx-auto text-secondary-300 dark:text-secondary-600 mb-4" />
        <h3 class="text-lg font-medium text-secondary-900 dark:text-white mb-2">Posting not found</h3>
        <a href="/postings"><Button variant="primary">Back to Postings</Button></a>
      </div>
    </Card>
  {:else}
    <div class="grid gap-6 lg:grid-cols-3">
      <!-- Main Content -->
      <div class="lg:col-span-2 space-y-6">
        <Card>
          <div class="p-6 space-y-4">
            <div class="flex items-start justify-between">
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-2">
                  <h1 class="text-2xl font-bold text-secondary-900 dark:text-white">{posting.title}</h1>
                  <span class={cn('px-3 py-1 text-sm font-medium rounded-full', getStatusColor(posting.status))}>
                    {posting.status.charAt(0).toUpperCase() + posting.status.slice(1)}
                  </span>
                </div>
                <p class="text-xl text-secondary-600 dark:text-secondary-400">{posting.company}</p>
              </div>
            </div>

            <div class="flex flex-wrap items-center gap-4 text-sm text-secondary-600 dark:text-secondary-400 border-t border-b border-secondary-200 dark:border-secondary-700 py-4">
              {#if posting.location}
                <span class="flex items-center gap-1"><MapPin class="h-4 w-4" /> {posting.location}</span>
              {/if}
              {#if posting.remote_policy}
                <span class="flex items-center gap-1"><Globe class="h-4 w-4" /> {posting.remote_policy}</span>
              {/if}
              {#if posting.job_type}
                <span class="flex items-center gap-1"><Briefcase class="h-4 w-4" /> {posting.job_type}</span>
              {/if}
              {#if posting.salary_range}
                <span class="flex items-center gap-1"><DollarSign class="h-4 w-4" /> {posting.salary_range}</span>
              {/if}
              {#if posting.deadline}
                <span class="flex items-center gap-1 text-red-600"><Clock class="h-4 w-4" /> Deadline: {formatDate(posting.deadline)}</span>
              {/if}
              {#if posting.application_email}
                <span class="flex items-center gap-1"><Mail class="h-4 w-4" /> {posting.application_email}</span>
              {/if}
            </div>

            {#if posting.summary}
              <div class="prose dark:prose-invert max-w-none">
                <h3 class="font-semibold mb-2">Summary</h3>
                <p class="whitespace-pre-wrap">{posting.summary}</p>
              </div>
            {/if}

            <div class="prose dark:prose-invert max-w-none">
              <h3 class="font-semibold mb-2">Description</h3>
              <div class="whitespace-pre-wrap">{posting.description}</div>
            </div>

            {#if posting.skills.length > 0}
              <div>
                <h3 class="font-semibold mb-2">Required Skills</h3>
                <div class="flex flex-wrap gap-2">
                  {#each posting.skills as skill}
                    <span class="px-3 py-1 bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300 rounded-full text-sm">{skill}</span>
                  {/each}
                </div>
              </div>
            {/if}

            <div class="grid gap-4 md:grid-cols-2">
              {#if posting.experience_required}
                <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
                  <h4 class="font-medium text-secondary-900 dark:text-white mb-1">Experience Required</h4>
                  <p class="text-secondary-600 dark:text-secondary-400">{posting.experience_required}</p>
                </div>
              {/if}
              {#if posting.education_required}
                <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
                  <h4 class="font-medium text-secondary-900 dark:text-white mb-1">Education Required</h4>
                  <p class="text-secondary-600 dark:text-secondary-400">{posting.education_required}</p>
                </div>
              {/if}
              {#if posting.languages.length > 0}
                <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
                  <h4 class="font-medium text-secondary-900 dark:text-white mb-1">Languages</h4>
                  <p class="text-secondary-600 dark:text-secondary-400">{posting.languages.join(', ')}</p>
                </div>
              {/if}
              {#if posting.contact_person}
                <div class="p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
                  <h4 class="font-medium text-secondary-900 dark:text-white mb-1">Contact Person</h4>
                  <p class="text-secondary-600 dark:text-secondary-400">{posting.contact_person}</p>
                  {#if posting.contact_phone}
                    <p class="text-secondary-600 dark:text-secondary-400">{posting.contact_phone}</p>
                  {/if}
                </div>
              {/if}
            </div>
          </div>
        </Card>

        {#if posting.application_steps.length > 0}
          <Card>
            <div class="p-6">
              <h2 class="text-lg font-semibold mb-4">Application Steps</h2>
              <div class="space-y-4">
                {#each posting.application_steps as step}
                  <div class="flex items-start gap-4 p-4 bg-secondary-50 dark:bg-secondary-800/50 rounded-lg">
                    <div class="flex-shrink-0 w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center text-primary-700 dark:text-primary-300 font-bold">
                      {step.order}
                    </div>
                    <div class="flex-1">
                      <p class="font-medium text-secondary-900 dark:text-white">{step.description}</p>
                      <div class="flex items-center gap-3 mt-2 text-sm text-secondary-600 dark:text-secondary-400">
                        <span class="px-2 py-0.5 bg-secondary-200 dark:bg-secondary-700 rounded">{step.action_type}</span>
                        <a href={step.target} target="_blank" class="text-primary-600 hover:underline flex items-center gap-1">
                          <ExternalLink class="h-3.5 w-3.5" />
                          {step.target}
                        </a>
                        {#if step.deadline}
                          <span class="text-red-600"><Clock class="h-3.5 w-3.5 inline" /> Due: {formatDate(step.deadline)}</span>
                        {/if}
                        {#if step.required}
                          <span class="px-2 py-0.5 bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 rounded text-xs">Required</span>
                        {/if}
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          </Card>
        {/if}
      </div>

      <!-- Sidebar -->
      <div class="space-y-6">
        <Card>
          <div class="p-6 space-y-4">
            <h3 class="font-semibold">Details</h3>
            <dl class="space-y-3 text-sm">
              <div class="flex justify-between">
                <dt class="text-secondary-600 dark:text-secondary-400">Source</dt>
                <dd class="font-medium text-secondary-900 dark:text-white truncate max-w-[150px]">{posting.source_id}</dd>
              </div>
              <div class="flex justify-between">
                <dt class="text-secondary-600 dark:text-secondary-400">External ID</dt>
                <dd class="font-medium text-secondary-900 dark:text-white font-mono truncate max-w-[150px]">{posting.source_posting_id}</dd>
              </div>
              <div class="flex justify-between">
                <dt class="text-secondary-600 dark:text-secondary-400">Scraped</dt>
                <dd class="font-medium text-secondary-900 dark:text-white">{formatDate(posting.scraped_at, {year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'})}</dd>
              </div>
              {#if posting.posted_date}
                <div class="flex justify-between">
                  <dt class="text-secondary-600 dark:text-secondary-400">Posted</dt>
                  <dd class="font-medium text-secondary-900 dark:text-white">{formatDate(posting.posted_date)}</dd>
                </div>
              {/if}
              <div class="flex justify-between">
                <dt class="text-secondary-600 dark:text-secondary-400">Content Hash</dt>
                <dd class="font-medium text-secondary-900 dark:text-white font-mono text-xs truncate max-w-[150px]">{posting.content_hash}</dd>
              </div>
            </dl>

            <div class="pt-4 border-t border-secondary-200 dark:border-secondary-700">
              <a href={posting.application_url} target="_blank" class="block w-full">
                <Button variant="primary" class="w-full">
                  <ExternalLink class="h-4 w-4 mr-2" />
                  View Original Posting
                </Button>
              </a>
              {#if posting.application_email}
                <a href={`mailto:${posting.application_email}`} class="block w-full mt-2">
                  <Button variant="secondary" class="w-full">
                    <Mail class="h-4 w-4 mr-2" />
                    Apply via Email
                  </Button>
                </a>
              {/if}
            </div>
          </div>
        </Card>

        {#if posting.raw_html}
          <Card>
            <div class="p-6">
              <h3 class="font-semibold mb-2">Raw HTML (Debug)</h3>
              <div class="max-h-64 overflow-auto p-4 bg-secondary-100 dark:bg-secondary-800 rounded font-mono text-xs whitespace-pre-wrap text-secondary-600 dark:text-secondary-400">
                {posting.raw_html.slice(0, 3000)}{posting.raw_html.length > 3000 ? '... (truncated)' : ''}
              </div>
            </div>
          </Card>
        {/if}
      </div>
    </div>
  {/if}
</div>

<Modal open={showReExtractModal} title="Re-extract with LLM" on:close={() => showReExtractModal = false}>
  <p class="text-secondary-600 dark:text-secondary-400 mb-4">This will re-run the LLM extraction on the raw content. Make sure LLM settings are configured in Settings.</p>
  <div class="flex justify-end gap-3">
    <Button variant="secondary" on:click={() => showReExtractModal = false}>Cancel</Button>
    <Button variant="primary" on:click={handleReExtract} disabled={reExtractLoading}>
      {reExtractLoading ? 'Extracting...' : 'Re-extract'}
    </Button>
  </div>
</Modal>