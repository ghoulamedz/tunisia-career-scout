// Global stores using Svelte 5 runes

import { invoke } from '@tauri-apps/api/core';
import { routes, getRoute } from './routes';
import type { Source, SourceType, SourceConfig, SourceStatus, JobPosting, CVData, CVProfile, CVTemplate, CVTheme, MatchResult, Settings, LLMSettings, PushSettings, SyncSettings, ScanLog } from './types';

// Page store for routing
export const page = $state({
  url: typeof window !== 'undefined' ? window.location.pathname + window.location.search : '/',
  params: {} as Record<string, string>,
  navigate(url: string) {
    page.url = url;
    window.history.pushState({}, '', url);
    page.updateParams();
  },
  updateParams() {
    // Use getRoute to match URL and extract params
    const route = getRoute(page.url);
    page.params = route.params;
  },
  init() {
    // Initialize from current URL
    page.updateParams();
    // Listen for browser back/forward
    window.addEventListener('popstate', () => {
      page.url = window.location.pathname + window.location.search;
      page.updateParams();
    });
  },
});

// Initialize page store
if (typeof window !== 'undefined') {
  page.init();
}

// Sources store
export const sourcesStore = $state({
  items: [] as Source[],
  loading: false,
  error: null as string | null,

  async load(enabledOnly = false) {
    this.loading = true;
    this.error = null;
    try {
      this.items = await invoke('list_sources', { enabledOnly });
    } catch (e) {
      this.error = e as string;
    } finally {
      this.loading = false;
    }
  },

  async create(source: Omit<Source, 'id' | 'created_at' | 'updated_at' | 'last_scan_at' | 'last_error' | 'status'>) {
    const newSource = await invoke('create_source', source);
    this.items = [newSource, ...this.items];
    return newSource;
  },

  async update(id: string, updates: Partial<Source>) {
    const updated = await invoke('update_source', { id, ...updates });
    const idx = this.items.findIndex(s => s.id === id);
    if (idx !== -1) this.items[idx] = updated;
    return updated;
  },

  async delete(id: string) {
    await invoke('delete_source', { id });
    this.items = this.items.filter(s => s.id !== id);
  },

  async scan(id: string) {
    return await invoke('trigger_source_scan', { id });
  },

  async getLogs(sourceId: string, limit = 50) {
    return await invoke('get_source_logs', { sourceId, limit });
  },
});

// Postings store
export const postingsStore = $state({
  items: [] as JobPosting[],
  loading: false,
  error: null as string | null,
  total: 0,
  currentPage: 1,
  pageSize: 20,
  filters: {
    sourceId: null as string | null,
    status: null as string | null,
    query: '',
  },

  async load() {
    this.loading = true;
    this.error = null;
    try {
      this.items = await invoke('list_postings', {
        sourceId: this.filters.sourceId || undefined,
        status: this.filters.status || undefined,
        limit: this.pageSize,
        offset: (this.currentPage - 1) * this.pageSize,
      });
    } catch (e) {
      this.error = e as string;
    } finally {
      this.loading = false;
    }
  },

  async search(query: string) {
    this.filters.query = query;
    this.currentPage = 1;
    try {
      this.items = await invoke('search_postings', { query, limit: this.pageSize });
    } catch (e) {
      this.error = e as string;
    }
  },

  async get(id: string) {
    return await invoke('get_posting', { id });
  },

  async reExtract(id: string, llmSettings: LLMSettings) {
    return await invoke('re_extract_posting', { id, llmSettings });
  },

  async updateStatus(id: string, status: string) {
    await invoke('update_posting_status', { id, status });
    const idx = this.items.findIndex(p => p.id === id);
    if (idx !== -1) this.items[idx].status = status as any;
  },
});

// CV store
export const cvStore = $state({
  profiles: [] as CVProfile[],
  current: null as CVProfile | null,
  loading: false,
  error: null as string | null,

  async load() {
    this.loading = true;
    try {
      this.profiles = await invoke('list_cvs');
      const def = this.profiles.find(p => p.is_default);
      this.current = def || this.profiles[0] || null;
    } catch (e) {
      this.error = e as string;
    } finally {
      this.loading = false;
    }
  },

  async create(profile: Omit<CVProfile, 'id' | 'created_at' | 'updated_at'>) {
    const newProfile = await invoke('save_cv', profile);
    this.profiles = [newProfile, ...this.profiles];
    if (newProfile.is_default) this.current = newProfile;
    return newProfile;
  },

  async update(id: string, updates: Partial<CVProfile>) {
    const updated = await invoke('update_cv', { id, ...updates });
    const idx = this.profiles.findIndex(p => p.id === id);
    if (idx !== -1) this.profiles[idx] = updated;
    if (this.current?.id === id) this.current = updated;
    return updated;
  },

  async delete(id: string) {
    await invoke('delete_cv', { id });
    this.profiles = this.profiles.filter(p => p.id !== id);
    if (this.current?.id === id) this.current = this.profiles[0] || null;
  },

  async setDefault(id: string) {
    const updated = await invoke('set_default_cv', { id });
    const idx = this.profiles.findIndex(p => p.id === id);
    if (idx !== -1) this.profiles[idx] = updated;
    this.current = updated;
    // Update other profiles to not be default
    this.profiles = this.profiles.map(p => p.id === id ? updated : { ...p, is_default: false });
    return updated;
  },

  async generatePDF(cvId: string) {
    return await invoke('generate_pdf', { cvId });
  },

  async preview(template: CVTemplate, data: CVData) {
    return await invoke('preview_template', { template, data });
  },

  async listTemplates() {
    return await invoke('list_templates');
  },
});

// Matches store
export const matchesStore = $state({
  items: [] as MatchResult[],
  loading: false,
  error: null as string | null,
  cvProfileId: null as string | null,

  async load(cvProfileId: string, minScore = 0) {
    this.cvProfileId = cvProfileId;
    this.loading = true;
    this.error = null;
    try {
      this.items = await invoke('get_match_results', { cvProfileId, minScore });
    } catch (e) {
      this.error = e as string;
    } finally {
      this.loading = false;
    }
  },

  async runMatching(cvProfileId: string, minScore = 0, limit = 100) {
    this.loading = true;
    try {
      this.items = await invoke('match_cv_to_postings', { cvProfileId, minScore, limit });
    } catch (e) {
      this.error = e as string;
    } finally {
      this.loading = false;
    }
  },

  async rerank(llmSettings: LLMSettings) {
    if (!this.cvProfileId) return;
    this.loading = true;
    try {
      this.items = await invoke('rerank_matches', { cvProfileId: this.cvProfileId, llmSettings });
    } catch (e) {
      this.error = e as string;
    } finally {
      this.loading = false;
    }
  },
});

// Settings store
export const settingsStore = $state({
  data: null as Settings | null,
  loading: false,
  error: null as string | null,

  async load() {
    this.loading = true;
    try {
      this.data = await invoke('get_settings');
    } catch (e) {
      this.error = e as string;
    } finally {
      this.loading = false;
    }
  },

  async update(settings: Partial<Settings>) {
    const updated = { ...this.data!, ...settings };
    await invoke('update_settings', { settings: updated });
    this.data = updated;
  },

  async updateLLM(settings: LLMSettings) {
    await invoke('update_llm_settings', { settings });
    if (this.data) this.data.llm = settings;
  },

  async updatePush(settings: PushSettings) {
    await invoke('update_push_settings', { settings });
    if (this.data) this.data.push = settings;
  },

  async updateSync(settings: SyncSettings) {
    await invoke('update_sync_settings', { settings });
    if (this.data) this.data.sync = settings;
  },
});

// Toast store
export const toasts = $state([] as Array<{ id: string; type: 'success' | 'error' | 'warning' | 'info'; message: string }>);

export function toast(type: 'success' | 'error' | 'warning' | 'info', message: string) {
  const id = Math.random().toString(36).slice(2);
  toasts.push({ id, type, message });
  setTimeout(() => {
    const idx = toasts.findIndex(t => t.id === id);
    if (idx !== -1) toasts.splice(idx, 1);
  }, 5000);
}