<script lang="ts">
  import { onMount } from 'svelte';
  import { settingsStore, toast } from '$lib/stores';
  import { settingsApi, gmailApi } from '$lib/api';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import { getLLMProviders } from '$lib/utils';
  import { Save, RefreshCw, Bell, Shield, Database, Cpu, Mail, Key, CheckCircle, AlertCircle, Loader2, GitBranch, Cloud } from '@lucide/svelte';
  import { cn } from '$lib/utils';
  import type { LLMProvider } from '$lib/types';

  let llmProvider = $state('ollama');
  let llmModel = $state('llama3.2');
  let llmBaseUrl = $state('http://localhost:11434');
  let llmApiKey = $state('');
  let llmTemperature = $state(0.3);
  let llmMaxTokens = $state(4096);
  let llmTimeout = $state(60);

  let pushEnabled = $state(false);
  let pushMinScore = $state(75);
  let pushQuietStart = $state('22:00');
  let pushQuietEnd = $state('08:00');

  let theme = $state('auto');
  let language = $state('en');
  let scanConcurrency = $state(3);
  let retentionDays = $state(90);

  // Sync settings state
  let syncEnabled = $state(false);
  let syncMethod = $state('file-export');
  let syncPeerId = $state('');

  // Gmail OAuth state
  let gmailClientId = $state('');
  let gmailClientSecret = $state('');
  let gmailRedirectUri = $state('http://localhost:0/callback');
  let gmailAndroidScheme = $state('com.tunisiacareerscout:/oauth2redirect');
  let gmailConfigured = $state(false);
  let gmailConnected = $state(false);
  let gmailStatus = $state<{ authenticated: boolean; expires_in_seconds: number; has_refresh_token: boolean } | null>(null);
  let gmailAuthLoading = $state(false);
  let gmailAuthUrl = $state('');
  let gmailAuthState = $state('');
  let gmailError = $state('');

  let saving = $state(false);
  let activeTab = $state('llm');

  const tabs = [
    { id: 'llm', label: 'LLM Settings', icon: Cpu },
    { id: 'push', label: 'Push Notifications', icon: Bell },
    { id: 'sync', label: 'Sync', icon: GitBranch },
    { id: 'gmail', label: 'Gmail OAuth', icon: Mail },
    { id: 'general', label: 'General', icon: Shield },
    { id: 'advanced', label: 'Advanced', icon: Database },
  ];

  async function loadSettings() {
    const settings = await settingsApi.get();
    llmProvider = settings.llm.provider;
    llmModel = settings.llm.model;
    llmBaseUrl = settings.llm.base_url || '';
    llmApiKey = settings.llm.api_key || '';
    llmTemperature = settings.llm.temperature;
    llmMaxTokens = settings.llm.max_tokens || 4096;
    llmTimeout = settings.llm.timeout_seconds;

    pushEnabled = settings.push.enabled;
    pushMinScore = settings.push.min_match_score;
    pushQuietStart = settings.push.quiet_hours_start || '22:00';
    pushQuietEnd = settings.push.quiet_hours_end || '08:00';

    theme = settings.theme;
    language = settings.language;
    scanConcurrency = settings.scan_concurrency;
    retentionDays = settings.retention_days;

    // Load Sync settings
    syncEnabled = settings.sync.enabled;
    syncMethod = settings.sync.method;
    syncPeerId = settings.sync.peer_id || '';

    // Load Gmail settings
    gmailClientId = settings.gmail_client_id || '';
    gmailClientSecret = settings.gmail_client_secret || '';
    gmailRedirectUri = settings.gmail_redirect_uri || 'http://localhost:0/callback';
    gmailAndroidScheme = settings.gmail_android_scheme || 'com.tunisiacareerscout:/oauth2redirect';
    gmailConfigured = !!(settings.gmail_client_id && settings.gmail_client_secret);

    // Check Gmail connection status
    if (gmailConfigured) {
      await checkGmailStatus();
    }
  }

  async function checkGmailStatus() {
    try {
      const status = await gmailApi.status();
      gmailStatus = status;
      gmailConnected = status.authenticated;
    } catch (e) {
      console.error('Failed to check Gmail status:', e);
      gmailConnected = false;
      gmailStatus = null;
    }
  }

  async function saveLLMSettings() {
    saving = true;
    try {
      await settingsApi.updateLLM({
        provider: llmProvider,
        model: llmModel,
        base_url: llmBaseUrl || undefined,
        api_key: llmApiKey || undefined,
        temperature: llmTemperature,
        max_tokens: llmMaxTokens,
        timeout_seconds: llmTimeout,
      });
      toast('success', 'LLM settings saved');
    } catch (e) {
      toast('error', 'Failed to save LLM settings');
    } finally {
      saving = false;
    }
  }

  async function savePushSettings() {
    saving = true;
    try {
      await settingsApi.updatePush({
        enabled: pushEnabled,
        min_match_score: pushMinScore,
        quiet_hours_start: pushQuietStart || undefined,
        quiet_hours_end: pushQuietEnd || undefined,
      });
      toast('success', 'Push settings saved');
    } catch (e) {
      toast('error', 'Failed to save push settings');
    } finally {
      saving = false;
    }
  }

  async function saveGeneralSettings() {
    saving = true;
    try {
      await settingsApi.update({
        theme,
        language,
        scan_concurrency: scanConcurrency,
        retention_days: retentionDays,
      });
      toast('success', 'General settings saved');
    } catch (e) {
      toast('error', 'Failed to save general settings');
    } finally {
      saving = false;
    }
  }

  async function saveSyncSettings() {
    saving = true;
    try {
      await settingsApi.updateSync({
        enabled: syncEnabled,
        method: syncMethod,
        peer_id: syncPeerId || undefined,
      });
      toast('success', 'Sync settings saved');
    } catch (e) {
      toast('error', 'Failed to save sync settings');
    } finally {
      saving = false;
    }
  }

  async function saveGmailSettings() {
    saving = true;
    try {
      await settingsApi.update({
        gmail_client_id: gmailClientId || undefined,
        gmail_client_secret: gmailClientSecret || undefined,
        gmail_redirect_uri: gmailRedirectUri || undefined,
        gmail_android_scheme: gmailAndroidScheme || undefined,
      });
      gmailConfigured = !!(gmailClientId && gmailClientSecret);
      toast('success', 'Gmail settings saved');
      if (gmailConfigured) {
        await checkGmailStatus();
      }
    } catch (e) {
      toast('error', 'Failed to save Gmail settings');
    } finally {
      saving = false;
    }
  }

  async function startGmailAuth() {
    if (!gmailConfigured) {
      toast('error', 'Please configure Gmail credentials first');
      return;
    }
    gmailAuthLoading = true;
    gmailError = '';
    try {
      const result = await gmailApi.authStart();
      gmailAuthUrl = result.auth_url;
      gmailAuthState = result.state;
      // Open auth URL in browser
      if (typeof window !== 'undefined') {
        window.open(gmailAuthUrl, '_blank', 'width=500,height=600');
      }
      toast('info', 'Opening browser for Gmail authentication...');
    } catch (e: any) {
      gmailError = e.message || 'Failed to start authentication';
      toast('error', gmailError);
    } finally {
      gmailAuthLoading = false;
    }
  }

  async function disconnectGmail() {
    gmailAuthLoading = true;
    try {
      await gmailApi.disconnect();
      gmailConnected = false;
      gmailStatus = null;
      toast('success', 'Gmail disconnected');
    } catch (e: any) {
      toast('error', e.message || 'Failed to disconnect Gmail');
    } finally {
      gmailAuthLoading = false;
    }
  }

  async function testLLMConnection() {
    saving = true;
    try {
      // Would call a test endpoint
      toast('success', 'LLM connection test passed (mock)');
    } catch (e) {
      toast('error', 'LLM connection failed');
    } finally {
      saving = false;
    }
  }

  async function registerPush() {
    // Would request notification permission and get FCM token
    toast('info', 'Push registration not yet implemented');
  }

  onMount(loadSettings);
</script>

<div class="max-w-4xl mx-auto space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-bold text-secondary-900 dark:text-white">Settings</h1>
      <p class="text-secondary-600 dark:text-secondary-400">Configure your application preferences</p>
    </div>
  </div>

  <!-- Tabs -->
  <div class="flex border-b border-secondary-200 dark:border-secondary-700">
    {#each tabs as tab}
      <button
        on:click={() => activeTab = tab.id}
        class={cn(
          'px-4 py-3 text-sm font-medium border-b-2 transition-colors -mb-px flex items-center gap-2',
          activeTab === tab.id
            ? 'border-primary-500 text-primary-600 dark:text-primary-400'
            : 'border-transparent text-secondary-500 hover:text-secondary-700 dark:hover:text-secondary-300'
        )}
      >
        <svelte:component this={tab.icon} class="h-4 w-4" />
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- LLM Settings -->
  {#if activeTab === 'llm'}
    <Card>
      <div class="p-6 space-y-6">
        <div class="flex items-center justify-between">
          <h2 class="text-lg font-semibold">LLM Configuration</h2>
          <Button variant="secondary" on:click={testLLMConnection} disabled={saving}>
            <RefreshCw class="h-4 w-4 mr-2" />
            Test Connection
          </Button>
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Provider</label>
            <select
              bind:value={llmProvider}
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            >
              <option value="ollama">Ollama (Local)</option>
              <option value="openai">OpenAI</option>
              <option value="openai-compatible">OpenAI Compatible</option>
              <option value="llama-cpp">llama.cpp</option>
            </select>
          </div>

          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Model</label>
            <input
              bind:value={llmModel}
              type="text"
              placeholder="llama3.2, gpt-4o-mini, etc."
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <div class="md:col-span-2">
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Base URL</label>
            <input
              bind:value={llmBaseUrl}
              type="url"
              placeholder="http://localhost:11434 (Ollama) or https://api.openai.com/v1"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">API Key</label>
            <input
              bind:value={llmApiKey}
              type="password"
              placeholder="sk-... (if required)"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            <p class="text-xs text-secondary-500 mt-1">Leave empty for local models</p>
          </div>
        </div>

        <div class="grid gap-4 md:grid-cols-3">
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Temperature</label>
            <input
              type="range"
              bind:value={llmTemperature}
              min="0"
              max="2"
              step="0.1"
              class="w-full h-2 bg-secondary-200 dark:bg-secondary-700 rounded-lg appearance-none cursor-pointer accent-primary-600"
            />
            <p class="text-sm text-secondary-600 dark:text-secondary-400 mt-1 text-center">{llmTemperature}</p>
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Max Tokens</label>
            <input
              bind:value={llmMaxTokens}
              type="number"
              min="100"
              max="32000"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Timeout (seconds)</label>
            <input
              bind:value={llmTimeout}
              type="number"
              min="10"
              max="300"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>
        </div>

        <div class="pt-4 border-t border-secondary-200 dark:border-secondary-700">
          <Button variant="primary" on:click={saveLLMSettings} disabled={saving} class="w-full sm:w-auto">
            <Save class="h-4 w-4 mr-2" />
            {saving ? 'Saving...' : 'Save LLM Settings'}
          </Button>
        </div>
      </div>
    </Card>

  {:else if activeTab === 'push'}
    <Card>
      <div class="p-6 space-y-6">
        <h2 class="text-lg font-semibold">Push Notifications</h2>

        <div class="flex items-center justify-between">
          <div>
            <h3 class="font-medium">Enable Push Notifications</h3>
            <p class="text-sm text-secondary-600 dark:text-secondary-400">Receive notifications for high-match job postings</p>
          </div>
          <label class="relative inline-flex items-center cursor-pointer">
            <input type="checkbox" bind:value={pushEnabled} class="sr-only peer" />
            <div class="w-11 h-6 bg-secondary-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 dark:peer-focus:ring-primary-800 rounded-full peer dark:bg-secondary-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-secondary-600 peer-checked:bg-primary-600"></div>
          </label>
        </div>

        {#if pushEnabled}
          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Minimum Match Score</label>
              <input
                type="range"
                bind:value={pushMinScore}
                min="0"
                max="100"
                step="5"
                class="w-full h-2 bg-secondary-200 dark:bg-secondary-700 rounded-lg appearance-none cursor-pointer accent-primary-600"
              />
              <p class="text-sm text-secondary-600 dark:text-secondary-400 mt-1">Notify only for matches above {pushMinScore}%</p>
            </div>

            <div class="grid gap-4 md:grid-cols-2">
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Quiet Hours Start</label>
                <input bind:value={pushQuietStart} type="time" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
              </div>
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Quiet Hours End</label>
                <input bind:value={pushQuietEnd} type="time" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
              </div>
            </div>

            <div class="flex items-center justify-between">
              <div>
                <h3 class="font-medium">FCM Token</h3>
                <p class="text-sm text-secondary-600 dark:text-secondary-400">Required for Android push notifications</p>
              </div>
              <Button variant="secondary" on:click={registerPush}>
                Register Device
              </Button>
            </div>
          </div>
        {/if}

        <div class="pt-4 border-t border-secondary-200 dark:border-secondary-700">
          <Button variant="primary" on:click={savePushSettings} disabled={saving}>
            <Save class="h-4 w-4 mr-2" />
            {saving ? 'Saving...' : 'Save Push Settings'}
          </Button>
        </div>
      </div>
    </Card>

  {:else if activeTab === 'gmail'}
    <Card>
      <div class="p-6 space-y-6">
        <h2 class="text-lg font-semibold">Gmail OAuth Configuration</h2>

        <div class="p-4 bg-primary-50 dark:bg-primary-900/30 border border-primary-200 dark:border-primary-800 rounded-lg">
          <h3 class="font-medium text-primary-900 dark:text-primary-100 mb-2">Required Scopes</h3>
          <ul class="text-sm text-primary-800 dark:text-primary-200 space-y-1">
            <li>• <code>https://www.googleapis.com/auth/gmail.readonly</code> - Read emails</li>
            <li>• <code>https://www.googleapis.com/auth/gmail.modify</code> - Manage labels (mark as read, archive)</li>
          </ul>
          <p class="text-xs text-primary-700 dark:text-primary-300 mt-2">Configure OAuth credentials in Google Cloud Console. See <a href="https://developers.google.com/gmail/api/auth/scopes" target="_blank" class="underline">Gmail API scopes</a>.</p>
        </div>

        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Client ID</label>
            <input
              bind:value={gmailClientId}
              type="text"
              placeholder="xxxxxxxxxx-xxxxxxxxxxxx.apps.googleusercontent.com"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            <p class="text-xs text-secondary-500 mt-1">From Google Cloud Console > APIs & Services > Credentials</p>
          </div>

          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Client Secret</label>
            <input
              bind:value={gmailClientSecret}
              type="password"
              placeholder="GOCSPX-xxxxxxxxxxxxx"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>

          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Redirect URI (Desktop)</label>
            <input
              bind:value={gmailRedirectUri}
              type="text"
              placeholder="http://localhost:0/callback"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            <p class="text-xs text-secondary-500 mt-1">Use <code>http://localhost:0/callback</code> for random port, or specific port like <code>http://localhost:8080/callback</code></p>
          </div>

          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Android Custom Scheme</label>
            <input
              bind:value={gmailAndroidScheme}
              type="text"
              placeholder="com.tunisiacareerscout:/oauth2redirect"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            <p class="text-xs text-secondary-500 mt-1">Custom URL scheme for Android deep linking (must match AndroidManifest.xml)</p>
          </div>
        </div>

        <div class="pt-4 border-t border-secondary-200 dark:border-secondary-700 flex gap-2">
          <Button variant="primary" on:click={saveGmailSettings} disabled={saving}>
            <Save class="h-4 w-4 mr-2" />
            {saving ? 'Saving...' : 'Save Gmail Settings'}
          </Button>
        </div>

        {#if gmailConfigured}
          <div class="pt-4 border-t border-secondary-200 dark:border-secondary-700">
            <h3 class="font-medium mb-4">Authentication Status</h3>

            {#if gmailConnected}
              <div class="p-4 bg-green-50 dark:bg-green-900/30 border border-green-200 dark:border-green-800 rounded-lg">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <CheckCircle class="h-5 w-5 text-green-600 dark:text-green-400" />
                    <div>
                      <p class="font-medium text-green-900 dark:text-green-100">Gmail Connected</p>
                      {#if gmailStatus}
                        <p class="text-sm text-green-700 dark:text-green-300">
                          Token expires in {Math.floor(gmailStatus.expires_in_seconds / 60)} minutes
                          {#if gmailStatus.has_refresh_token}
                            · Refresh token available
                          {/if}
                        </p>
                      {/if}
                    </div>
                  </div>
                  <Button variant="destructive" on:click={disconnectGmail} disabled={gmailAuthLoading}>
                    {#if gmailAuthLoading}
                      <Loader2 class="h-4 w-4 animate-spin mr-2" /> Disconnecting...
                    {:else}
                      Disconnect
                    {/if}
                  </Button>
                </div>
              </div>
            {:else}
              <div class="p-4 bg-yellow-50 dark:bg-yellow-900/30 border border-yellow-200 dark:border-yellow-800 rounded-lg">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <AlertCircle class="h-5 w-5 text-yellow-600 dark:text-yellow-400" />
                    <div>
                      <p class="font-medium text-yellow-900 dark:text-yellow-100">Not Authenticated</p>
                      <p class="text-sm text-yellow-700 dark:text-yellow-300">Click "Connect Gmail" to authorize access</p>
                    </div>
                  </div>
                  <Button variant="primary" on:click={startGmailAuth} disabled={gmailAuthLoading}>
                    {#if gmailAuthLoading}
                      <Loader2 class="h-4 w-4 animate-spin mr-2" /> Connecting...
                    {:else}
                      <Key class="h-4 w-4 mr-2" /> Connect Gmail
                    {/if}
                  </Button>
                </div>
                {#if gmailError}
                  <p class="text-sm text-red-600 dark:text-red-400 mt-3">{gmailError}</p>
                {/if}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </Card>

  {:else if activeTab === 'sync'}
    <Card>
      <div class="p-6 space-y-6">
        <h2 class="text-lg font-semibold">Data Synchronization</h2>

        <div class="flex items-center justify-between">
          <div>
            <h3 class="font-medium">Enable Sync</h3>
            <p class="text-sm text-secondary-600 dark:text-secondary-400">Synchronize data across devices</p>
          </div>
          <label class="relative inline-flex items-center cursor-pointer">
            <input type="checkbox" bind:value={syncEnabled} class="sr-only peer" />
            <div class="w-11 h-6 bg-secondary-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 dark:peer-focus:ring-primary-800 rounded-full peer dark:bg-secondary-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-secondary-600 peer-checked:bg-primary-600"></div>
          </label>
        </div>

        {#if syncEnabled}
          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Sync Method</label>
              <select
                bind:value={syncMethod}
                class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              >
                <option value="local-network">Local Network (P2P)</option>
                <option value="qr-code">QR Code Pairing</option>
                <option value="file-export">File Export/Import</option>
              </select>
              <p class="text-xs text-secondary-500 mt-1">Choose how to synchronize data between devices</p>
            </div>

            {#if syncMethod === 'local-network' || syncMethod === 'qr-code'}
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Peer ID</label>
                <input
                  bind:value={syncPeerId}
                  type="text"
                  placeholder="Auto-generated or enter manually"
                  class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                />
                <p class="text-xs text-secondary-500 mt-1">Unique identifier for this device in the sync network</p>
              </div>
            {/if}
          </div>
        {/if}

        <div class="pt-4 border-t border-secondary-200 dark:border-secondary-700">
          <Button variant="primary" on:click={saveSyncSettings} disabled={saving}>
            <Save class="h-4 w-4 mr-2" />
            {saving ? 'Saving...' : 'Save Sync Settings'}
          </Button>
        </div>
      </div>
    </Card>

  {:else if activeTab === 'general'}
    <Card>
      <div class="p-6 space-y-6">
        <h2 class="text-lg font-semibold">General Settings</h2>

        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Theme</label>
            <select
              bind:value={theme}
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            >
              <option value="light">Light</option>
              <option value="dark">Dark</option>
              <option value="auto">Auto (System)</option>
            </select>
          </div>

          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Language</label>
            <select
              bind:value={language}
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            >
              <option value="en">English</option>
              <option value="fr">French</option>
              <option value="ar">Arabic</option>
            </select>
          </div>
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Scan Concurrency</label>
            <input
              bind:value={scanConcurrency}
              type="number"
              min="1"
              max="10"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            <p class="text-xs text-secondary-500 mt-1">Max parallel source scans</p>
          </div>

          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Retention Days</label>
            <input
              bind:value={retentionDays}
              type="number"
              min="1"
              max="365"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            <p class="text-xs text-secondary-500 mt-1">Auto-delete old postings</p>
          </div>
        </div>

        <div class="pt-4 border-t border-secondary-200 dark:border-secondary-700">
          <Button variant="primary" on:click={saveGeneralSettings} disabled={saving}>
            <Save class="h-4 w-4 mr-2" />
            {saving ? 'Saving...' : 'Save General Settings'}
          </Button>
        </div>
      </div>
    </Card>

  {:else if activeTab === 'advanced'}
    <Card>
      <div class="p-6 space-y-6">
        <h2 class="text-lg font-semibold">Advanced</h2>

        <div class="space-y-4">
          <div class="p-4 border border-secondary-200 dark:border-secondary-700 rounded-lg">
            <h3 class="font-medium mb-2">Danger Zone</h3>
            <p class="text-sm text-secondary-600 dark:text-secondary-400 mb-4">These actions are irreversible.</p>
            <Button variant="destructive" on:click={() => settingsApi.reset()}>Reset All Settings</Button>
          </div>

          <div class="p-4 border border-secondary-200 dark:border-secondary-700 rounded-lg">
            <h3 class="font-medium mb-2">Database</h3>
            <p class="text-sm text-secondary-600 dark:text-secondary-400 mb-2">SQLite database location: <code class="text-xs bg-secondary-100 dark:bg-secondary-800 px-1.5 py-0.5 rounded">~/.local/share/tunisia-career-scout/app.db</code></p>
            <div class="flex gap-2">
              <Button variant="secondary">Backup Database</Button>
              <Button variant="secondary">View Schema</Button>
            </div>
          </div>

          <div class="p-4 border border-secondary-200 dark:border-secondary-700 rounded-lg">
            <h3 class="font-medium mb-2">Debug</h3>
            <div class="flex gap-2 flex-wrap">
              <Button variant="secondary">View Logs</Button>
              <Button variant="secondary">Export Config</Button>
              <Button variant="secondary">Import Config</Button>
            </div>
          </div>
        </div>
      </div>
    </Card>
  {/if}
</div>