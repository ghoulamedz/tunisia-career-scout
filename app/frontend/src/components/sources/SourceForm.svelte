<script lang="ts">
  import { onMount } from 'svelte';
  import { page, sourcesStore, toast, settingsStore } from '$lib/stores';
  import { sourcesApi, settingsApi } from '$lib/api';
  import { cn } from '$lib/utils';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import { SourceType, SourceConfig, PaginationConfig, AuthType } from '$lib/types';
  import { ChevronLeft } from '@lucide/svelte';

  interface Props {
    initialData?: any;
    onSuccess?: () => void;
    onCancel?: () => void;
  }

  let { initialData, onSuccess, onCancel }: Props = $props();

  let sourceType = $state<SourceType>('url');
  let name = $state('');
  let enabled = $state(true);
  let schedule = $state('0 * * * *');

  // URL config
  let url = $state('');
  let selector = $state('');
  let detailSelector = $state('');
  let paginationType: PaginationConfig['type'] = $state('url-param');
  let paginationSelector = $state('');
  let maxPages = $state(10);
  let headers = $state('{}');
  let headersPlaceholder = $state('{"User-Agent": "Custom Bot", "Authorization": "Bearer token"}');

  // RSS config
  let rssUrl = $state('');
  let itemXPath = $state('');
  let dateFormat = $state('');

  // Gmail config
  let labelIds = $state(['INBOX']);
  let gmailQuery = $state('');
  let maxResults = $state(50);
  let gmailAuthUrl = $state('');
  let gmailConnected = $state(false);
  let gmailConfigured = $state(false);

  // Social config
  let socialPlatform: 'facebook' | 'tiktok' | 'linkedin' = $state('linkedin');
  let socialTargetType: 'group' | 'page' | 'hashtag' | 'user' | 'search' = $state('search');
  let socialTargetId = $state('');
  let socialQuery = $state('');
  let socialLocation = $state('');
  let socialCookies = $state('');
  let socialCookiesPlaceholder = $state('[{"name": "session_id", "value": "abc123", "domain": ".linkedin.com"}]');
  let scrollDepth = $state(5);

  let loading = $state(false);
  let isEditing = $state(false);
  let editId = $state('');

  async function loadSource() {
    const id = initialData?.id || page.params.id;
    if (!id) return;

    isEditing = true;
    editId = id;
    try {
      const sources = await sourcesApi.list();
      const source = sources.find((s: any) => s.id === id);
      if (source) {
        populateForm(source);
      }
    } catch (e) {
      toast('error', 'Failed to load source');
    }
  }

  function populateForm(source: any) {
    name = source.name;
    sourceType = source.source_type;
    enabled = source.enabled;
    schedule = source.schedule;

    const config = source.config as SourceConfig;
    switch (source.source_type) {
      case 'url':
        url = config.url || '';
        selector = config.selector || '';
        detailSelector = config.detail_selector || '';
        if (config.pagination) {
          paginationType = config.pagination.type;
          paginationSelector = config.pagination.selector || '';
          maxPages = config.pagination.max_pages || 10;
        }
        headers = JSON.stringify(config.headers || {}, null, 2);
        break;
      case 'rss':
        rssUrl = config.url || '';
        itemXPath = config.item_xpath || '';
        dateFormat = config.date_format || '';
        break;
      case 'gmail':
        labelIds = config.label_ids || ['INBOX'];
        gmailQuery = config.query || '';
        maxResults = config.max_results || 50;
        gmailConnected = !!source.auth_data;
        break;
      case 'social':
        socialPlatform = config.platform || 'linkedin';
        const target = config.target;
        if (target) {
          socialTargetType = target.type;
          socialTargetId = target.id || target.tag || target.username || target.query || '';
          socialQuery = target.query || '';
          socialLocation = target.location || '';
        }
        socialCookies = config.cookies || '';
        scrollDepth = config.scroll_depth || 5;
        break;
    }
  }

  async function handleSubmit() {
    loading = true;
    try {
      let config: SourceConfig;

      switch (sourceType) {
        case 'url':
          config = {
            type: 'url',
            url,
            selector: selector || undefined,
            detail_selector: detailSelector || undefined,
            pagination: {
              type: paginationType,
              selector: paginationSelector || undefined,
              max_pages: maxPages,
            },
            headers: JSON.parse(headers),
          };
          break;
        case 'rss':
          config = {
            type: 'rss',
            url: rssUrl,
            item_xpath: itemXPath || undefined,
            date_format: dateFormat || undefined,
          };
          break;
        case 'gmail':
          config = {
            type: 'gmail',
            label_ids: labelIds,
            query: gmailQuery || undefined,
            max_results: maxResults,
          };
          break;
        case 'social':
          config = {
            type: 'social',
            platform: socialPlatform,
            target: {
              type: socialTargetType,
              id: socialTargetId,
              tag: socialTargetId,
              username: socialTargetId,
              query: socialQuery,
              location: socialLocation || undefined,
            },
            cookies: socialCookies || undefined,
            scroll_depth: scrollDepth,
          };
          break;
      }

      if (isEditing) {
        await sourcesApi.update(editId, { name, config, schedule, enabled });
        toast('success', 'Source updated');
      } else {
        await sourcesApi.create({ name, source_type: sourceType, config, schedule, enabled });
        toast('success', 'Source created');
      }
      onSuccess?.();
    } catch (e) {
      toast('error', `Failed to ${isEditing ? 'update' : 'create'} source`);
    } finally {
      loading = false;
    }
  }

  async function connectGmail() {
    try {
      gmailAuthUrl = await import('$lib/api').then(m => m.gmailApi.getAuthUrl());
      window.open(gmailAuthUrl, '_blank', 'width=500,height=600');
    } catch (e) {
      toast('error', 'Failed to get auth URL');
    }
  }

  onMount(async () => {
    if (initialData?.id || page.params.id) {
      loadSource();
    }
    try {
      gmailConfigured = await settingsApi.gmailConfigured();
    } catch {
      gmailConfigured = false;
    }
  });

  $effect(() => {
    if (!gmailConfigured && sourceType === 'gmail') {
      toast('warning', 'Gmail not configured — set GMAIL_CLIENT_ID/SECRET in .env to use Gmail sources');
    }
  });
</script>

<div class="max-w-3xl mx-auto space-y-6">
  <div class="flex items-center justify-between">
    <a href="/sources" class="flex items-center gap-2 text-secondary-600 hover:text-secondary-900 dark:text-secondary-400 dark:hover:text-white">
      <ChevronLeft class="h-5 w-5" />
      Back
    </a>
    <h1 class="text-2xl font-bold text-secondary-900 dark:text-white">{isEditing ? 'Edit Source' : 'Add Source'}</h1>
  </div>

  <Card>
    <div class="p-6 space-y-6">
      <div class="border-b border-secondary-200 dark:border-secondary-700 pb-4">
        <h2 class="text-lg font-semibold">Basic Information</h2>
      </div>

      <div class="grid gap-4 md:grid-cols-2">
        <div class="md:col-span-2">
          <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Name *</label>
          <input
            bind:value={name}
            type="text"
            placeholder="e.g., LinkedIn Jobs, Company Careers Page"
            class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            required
          />
        </div>

        <div>
          <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Source Type *</label>
          <select
            bind:value={sourceType}
            class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          >
            <option value="url">URL / Web Page</option>
            <option value="rss">RSS Feed</option>
            {#if gmailConfigured}
              <option value="gmail">Gmail (Job Alerts)</option>
            {:else}
              <option value="gmail" disabled>Gmail (Job Alerts) — Not configured</option>
            {/if}
            <option value="social">Social Media (FB/TikTok/LinkedIn)</option>
          </select>
          {#if !gmailConfigured}
            <p class="text-xs text-secondary-500 mt-1">Set GMAIL_CLIENT_ID and GMAIL_CLIENT_SECRET in .env to enable Gmail sources</p>
          {/if}
        </div>

        <div>
          <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Schedule (Cron) *</label>
          <input
            bind:value={schedule}
            type="text"
            placeholder="0 * * * * (hourly)"
            class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono"
            required
          />
          <p class="text-xs text-secondary-500 mt-1">Cron expression. Common: <code class="font-mono">0 * * * *</code> (hourly), <code class="font-mono">0 6 * * *</code> (daily 6AM), <code class="font-mono">0 6 * * 1</code> (weekly Monday 6AM)</p>
        </div>

        <div class="flex items-center gap-2">
          <input type="checkbox" bind:value={enabled} id="enabled" class="h-4 w-4 rounded border-secondary-300 text-primary-600 focus:ring-primary-500" />
          <label for="enabled" class="text-sm text-secondary-700 dark:text-secondary-300">Enabled</label>
        </div>
      </div>

      <!-- Type-specific config -->
      <div class="border-t border-secondary-200 dark:border-secondary-700 pt-6">
        <h2 class="text-lg font-semibold mb-4">Source Configuration</h2>

        {#if sourceType === 'url'}
          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">URL *</label>
              <input bind:value={url} type="url" placeholder="https://example.com/jobs" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" required />
            </div>

            <div class="grid gap-4 md:grid-cols-2">
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Listing Selector (CSS)</label>
                <input bind:value={selector} type="text" placeholder=".job-listing, article.job" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono" />
                <p class="text-xs text-secondary-500 mt-1">Selector for job listing elements on the page</p>
              </div>
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Detail Selector (CSS)</label>
                <input bind:value={detailSelector} type="text" placeholder=".job-detail, #job-content" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono" />
                <p class="text-xs text-secondary-500 mt-1">Selector for detail page content (if different from listing)</p>
              </div>
            </div>

            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Pagination</label>
              <div class="grid gap-4 md:grid-cols-4">
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Type</label>
                  <select bind:value={paginationType} class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent">
                    <option value="url-param">URL Parameter (?page=N)</option>
                    <option value="next-button">Next Button</option>
                    <option value="infinite-scroll">Infinite Scroll</option>
                    <option value="page-numbers">Page Numbers</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Selector</label>
                  <input bind:value={paginationSelector} type="text" placeholder=".next-page, a[rel=next]" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Max Pages</label>
                  <input bind:value={maxPages} type="number" min="1" max="100" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
                </div>
                <div class="md:col-span-4">
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Custom Headers (JSON)</label>
                  <textarea bind:value={headers} rows="3" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono text-sm" placeholder={headersPlaceholder}></textarea>
                </div>
              </div>
            </div>
          </div>

        {:else if sourceType === 'rss'}
          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">RSS Feed URL *</label>
              <input bind:value={rssUrl} type="url" placeholder="https://example.com/feed.xml" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" required />
            </div>
            <div class="grid gap-4 md:grid-cols-2">
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Item XPath</label>
                <input bind:value={itemXPath} type="text" placeholder="//item or //entry" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono" />
              </div>
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Date Format</label>
                <input bind:value={dateFormat} type="text" placeholder="RFC3339, RFC2822, or custom" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono" />
              </div>
            </div>
          </div>

        {:else if sourceType === 'gmail'}
          <div class="space-y-4">
            <div class="p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
              <h4 class="font-medium text-blue-900 dark:text-blue-100 mb-2">Gmail OAuth Setup</h4>
              <p class="text-sm text-blue-700 dark:text-blue-300 mb-3">Connect your Gmail account to fetch job alert emails via Gmail API.</p>
              {#if gmailConnected}
                <div class="flex items-center gap-2 text-green-700 dark:text-green-300">
                  <svg class="h-5 w-5" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" /></svg>
                  <span>Connected</span>
                </div>
              {:else}
                <Button variant="primary" class="" on:click={connectGmail}>
                  <svg class="h-4 w-4 mr-2" fill="currentColor" viewBox="0 0 24 24"><path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/><path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/></svg>
                  Connect Gmail
                </Button>
              {/if}
            </div>

            <div class="grid gap-4 md:grid-cols-2">
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Gmail Labels</label>
                <div class="flex flex-wrap gap-2">
                  {#each ['INBOX', 'JOB_ALERTS', 'LINKEDIN', 'INDEED', 'CUSTOM'] as label}
                    <label class="inline-flex items-center gap-2 cursor-pointer">
                      <input type="checkbox" value={label} bind:group={labelIds} class="h-4 w-4 rounded border-secondary-300 text-primary-600 focus:ring-primary-500" />
                      <span class="text-sm">{label}</span>
                    </label>
                  {/each}
                </div>
              </div>
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Additional Query</label>
                <input bind:value={gmailQuery} type="text" placeholder='subject:"job alert" OR subject:"new position"' class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
              </div>
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Max Results per Scan</label>
                <input bind:value={maxResults} type="number" min="1" max="500" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
              </div>
            </div>
          </div>

        {:else if sourceType === 'social'}
          <div class="space-y-4">
            <div class="grid gap-4 md:grid-cols-2">
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Platform</label>
                <select bind:value={socialPlatform} class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent">
                  <option value="facebook">Facebook</option>
                  <option value="tiktok">TikTok</option>
                  <option value="linkedin">LinkedIn</option>
                </select>
              </div>
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Target Type</label>
                <select bind:value={socialTargetType} class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent">
                  <option value="group">Group</option>
                  <option value="page">Page</option>
                  <option value="hashtag">Hashtag</option>
                  <option value="user">User Profile</option>
                  <option value="search">Search</option>
                </select>
              </div>
            </div>

            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Target ID / Query</label>
              <input
                bind:value={socialTargetId}
                type="text"
                placeholder={socialTargetType === 'search' ? 'software engineer jobs' : socialTargetType === 'hashtag' ? 'jobs' : 'group/page/user ID'}
                class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              />
              <p class="text-xs text-secondary-500 mt-1">
                {socialTargetType === 'group' && 'Facebook Group ID or LinkedIn Group ID'}
                {socialTargetType === 'page' && 'Facebook Page ID or LinkedIn Page ID'}
                {socialTargetType === 'hashtag' && 'Hashtag without # (e.g., jobs, hiring)'}
                {socialTargetType === 'user' && 'Username or profile ID'}
                {socialTargetType === 'search' && 'Search keywords (e.g., software engineer, remote jobs)'}
              </p>
            </div>

            {#if socialTargetType === 'search'}
              <div class="grid gap-4 md:grid-cols-2">
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Location (optional)</label>
                  <input bind:value={socialLocation} type="text" placeholder="Tunisia, Remote, Paris" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
                </div>
              </div>
            {/if}

            <div class="grid gap-4 md:grid-cols-2">
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Cookies (JSON, optional)</label>
                <textarea bind:value={socialCookies} rows="3" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono text-sm" placeholder={socialCookiesPlaceholder}></textarea>
                <p class="text-xs text-secondary-500 mt-1">Paste exported cookies from browser dev tools</p>
              </div>
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Scroll Depth</label>
                <input bind:value={scrollDepth} type="number" min="1" max="50" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
                <p class="text-xs text-secondary-500 mt-1">Number of scroll iterations to load more content</p>
              </div>
            </div>

            <div class="p-4 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
              <p class="text-sm text-amber-900 dark:text-amber-100"><strong>Note:</strong> Social scraping requires the Python sidecar (Playwright) to be installed. The app will open a browser window for you to log in and capture cookies on first run.</p>
            </div>
          </div>
        {/if}
      </div>

      <div class="flex justify-end gap-3 pt-4 border-t border-secondary-200 dark:border-secondary-700">
        <Button variant="secondary" class="" on:click={onCancel || (() => window.location.href = '/sources')}>Cancel</Button>
        <Button variant="primary" class="" on:click={handleSubmit} disabled={loading || !name}>
          {loading ? 'Saving...' : (isEditing ? 'Update' : 'Create')}
        </Button>
      </div>
    </div>
  </Card>
</div>