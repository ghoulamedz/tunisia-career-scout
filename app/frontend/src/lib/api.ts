// API wrapper for Tauri commands

import { invoke } from '@tauri-apps/api/core';

// Sources
export const sourcesApi = {
  list: (enabledOnly = false) => invoke('list_sources', { enabledOnly }),
  get: (id: string) => invoke('get_source', { id }),
  create: (data: any) => invoke('create_source', data),
  update: (id: string, data: any) => invoke('update_source', { id, ...data }),
  delete: (id: string) => invoke('delete_source', { id }),
  scan: (id: string) => invoke('trigger_source_scan', { id }),
  scanAll: () => invoke('scan_all_sources', {}),
  toggleEnabled: (id: string) => invoke('toggle_source_enabled', { id }),
  getLogs: (sourceId: string, limit = 50) => invoke('get_source_logs', { sourceId, limit }),
};

// Postings
export const postingsApi = {
  list: (params: { sourceId?: string; status?: string; limit?: number; offset?: number }) => invoke('list_postings', params),
  get: (id: string) => invoke('get_posting', { id }),
  delete: (id: string) => invoke('delete_posting', { id }),
  reExtract: (id: string, llmSettings: any) => invoke('re_extract_posting', { id, llmSettings }),
  search: (query: string, limit = 20) => invoke('search_postings', { query, limit }),
  updateStatus: (id: string, status: string) => invoke('update_posting_status', { id, status }),
};

// CV
export const cvApi = {
  list: () => invoke('list_cvs'),
  get: (id?: string) => invoke('get_cv', { id }),
  save: (data: any) => invoke('save_cv', data),
  update: (id: string, data: any) => invoke('update_cv', { id, ...data }),
  delete: (id: string) => invoke('delete_cv', { id }),
  setDefault: (id: string) => invoke('set_default_cv', { id }),
  generatePDF: (cvId: string) => invoke<ArrayBuffer>('generate_pdf', { cvId }),
  preview: (template: string, data: any) => invoke<ArrayBuffer>('preview_template', { template, data }),
  compilePreview: (data: any, template?: string) => invoke<ArrayBuffer>('compile_cv_preview', { cv_data: data, template }),
  listTemplates: () => invoke('list_templates'),
};

// Matching
export const matchingApi = {
  run: (cvProfileId: string, minScore = 0, limit = 100) => invoke('match_cv_to_postings', { cvProfileId, minScore, limit }),
  list: (cvProfileId: string, minScore = 0, limit = 50, offset = 0) => invoke('get_match_results', { cvProfileId, minScore, limit, offset }),
  rerank: (cvProfileId: string, llmSettings: any) => invoke('rerank_matches', { cvProfileId, llmSettings }),
};

// Gmail
export const gmailApi = {
  authStart: () => invoke('gmail_auth_start'),
  authCallback: (code: string, state: string) => invoke('gmail_auth_callback', { code, state }),
  androidCallback: (callbackUrl: string) => invoke('gmail_auth_android_callback', { callbackUrl }),
  disconnect: () => invoke('gmail_disconnect'),
  status: () => invoke('gmail_status'),
  // Legacy (for source scanning)
  listMessages: (sourceId: string, maxResults = 50) => invoke('list_messages', { sourceId, maxResults }),
  fetchMessage: (sourceId: string, messageId: string) => invoke('fetch_message', { sourceId, messageId }),
};

// Settings
export const settingsApi = {
  get: () => invoke('get_settings'),
  update: (settings: any) => invoke('update_settings', { settings }),
  updateLLM: (settings: any) => invoke('update_llm_settings', { settings }),
  updatePush: (settings: any) => invoke('update_push_settings', { settings }),
  updateSync: (settings: any) => invoke('update_sync_settings', { settings }),
  reset: () => invoke('reset_settings'),
  gmailConfigured: () => invoke('gmail_configured'),
};

// Push
export const pushApi = {
  register: (fcmToken: string) => invoke('register_device', { fcmToken }),
  unregister: () => invoke('unregister_device'),
  sendTest: (title: string, body: string) => invoke('send_test_notification', { title, body }),
};