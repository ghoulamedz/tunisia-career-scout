// TypeScript types mirroring Rust models

export type UUID = string;

export type SourceType = 'url' | 'rss' | 'gmail' | 'social';
export type SourceStatus = 'active' | 'paused' | 'error';
export type AuthType = 'none' | 'bearer' | 'cookie' | 'oauth2' | 'app-password';
export type PostingStatus = 'active' | 'expired' | 'filled' | 'hidden' | 'applied' | 'saved';
export type ActionType = 'link' | 'email' | 'form' | 'upload' | 'other';
export type PaginationType = 'infinite-scroll' | 'next-button' | 'page-numbers' | 'url-param';
export type SocialPlatform = 'facebook' | 'tiktok' | 'linkedin';
export type SocialTargetType = 'group' | 'page' | 'hashtag' | 'user' | 'search';
export type CVTemplate = 'modern' | 'classic' | 'minimal';
export type CVTheme = 'light' | 'dark' | 'auto';
export type SkillLevel = 'beginner' | 'intermediate' | 'advanced' | 'expert';
export type LanguageProficiency = 'native' | 'fluent' | 'professional' | 'conversational' | 'basic';
export type LLMProvider = 'openai' | 'openai-compatible' | 'ollama' | 'llama-cpp';
export type SyncMethod = 'local-network' | 'qr-code' | 'file-export';
export type MatchCategory = 'skills' | 'experience' | 'education' | 'location' | 'keywords' | 'language' | 'salary' | 'other';
export type ScanStatus = 'running' | 'completed' | 'failed' | 'partial';

export interface PaginationConfig {
  type: PaginationType;
  selector?: string;
  maxPages?: number;
}

export interface SourceConfig {
  type: SourceType;
  // URL
  url?: string;
  selector?: string;
  detailSelector?: string;
  pagination?: PaginationConfig;
  headers?: Record<string, string>;
  // RSS
  itemXPath?: string;
  dateFormat?: string;
  // Gmail
  labelIds?: string[];
  query?: string;
  maxResults?: number;
  // Social
  platform?: SocialPlatform;
  target?: SocialTarget;
  cookies?: string;
  scrollDepth?: number;
}

export interface SocialTarget {
  type: SocialTargetType;
  id?: string;
  tag?: string;
  username?: string;
  query?: string;
  location?: string;
}

export interface AuthData {
  authType: AuthType;
  token?: string;
  refreshToken?: string;
  cookies?: string;
  expiresAt?: string;
}

export interface Source {
  id: UUID;
  name: string;
  sourceType: SourceType;
  config: SourceConfig;
  schedule: string;
  enabled: boolean;
  status: SourceStatus;
  authData?: AuthData;
  lastScanAt?: string;
  lastError?: string;
  createdAt: string;
  updatedAt: string;
}

export interface ApplicationStep {
  order: number;
  description: string;
  actionType: ActionType;
  target: string;
  deadline?: string;
  required: boolean;
  completed: boolean;
}

export interface JobPosting {
  id: UUID;
  sourceId: UUID;
  sourcePostingId: string;
  title: string;
  company: string;
  description: string;
  summary?: string;
  skills: string[];
  experienceRequired?: string;
  educationRequired?: string;
  languages: string[];
  location?: string;
  remotePolicy?: string;
  jobType?: string;
  salaryRange?: string;
  deadline?: string;
  applicationSteps: ApplicationStep[];
  applicationEmail?: string;
  applicationUrl?: string;
  contactPerson?: string;
  contactPhone?: string;
  postedDate?: string;
  scrapedAt: string;
  contentHash: string;
  rawHtml?: string;
  status: PostingStatus;
  createdAt: string;
  updatedAt: string;
}

export interface Experience {
  id: UUID;
  title: string;
  company: string;
  location?: string;
  startDate: string;
  endDate?: string;
  current: boolean;
  description: string;
  skills: string[];
}

export interface Education {
  id: UUID;
  institution: string;
  degree: string;
  fieldOfStudy?: string;
  location?: string;
  startDate: string;
  endDate?: string;
  current: boolean;
  description?: string;
  grade?: string;
}

export interface Skill {
  id: UUID;
  name: string;
  category?: string;
  level?: SkillLevel;
  years?: number;
}

export interface Language {
  id: UUID;
  name: string;
  proficiency: LanguageProficiency;
}

export interface Project {
  id: UUID;
  name: string;
  description: string;
  url?: string;
  github?: string;
  technologies: string[];
  startDate?: string;
  endDate?: string;
}

export interface Certification {
  id: UUID;
  name: string;
  issuer: string;
  date: string;
  expiry?: string;
  url?: string;
  credentialId?: string;
}

export interface CVData {
  fullName: string;
  email: string;
  phone?: string;
  location?: string;
  linkedin?: string;
  github?: string;
  portfolio?: string;
  summary?: string;
  experience: Experience[];
  education: Education[];
  skills: Skill[];
  languages: Language[];
  projects: Project[];
  certifications: Certification[];
  template: CVTemplate;
  theme: CVTheme;
}

export interface CVProfile {
  id: UUID;
  name: string;
  data: CVData;
  template: CVTemplate;
  theme: CVTheme;
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface MatchReason {
  category: MatchCategory;
  description: string;
  weight: number;
  matched: boolean;
}

export interface MatchResult {
  id: UUID;
  cvProfileId: UUID;
  postingId: UUID;
  score: number;
  matchedSkills: string[];
  missingSkills: string[];
  reasons: MatchReason[];
  tfidfScore?: number;
  llmScore?: number;
  createdAt: string;
}

export interface LLMSettings {
  provider: LLMProvider;
  model: string;
  baseUrl?: string;
  apiKey?: string;
  temperature: number;
  maxTokens?: number;
  timeoutSeconds: number;
}

export interface PushSettings {
  enabled: boolean;
  fcmToken?: string;
  minMatchScore: number;
  quietHoursStart?: string;
  quietHoursEnd?: string;
}

export interface SyncSettings {
  enabled: boolean;
  method: SyncMethod;
  peerId?: string;
  lastSync?: string;
}

export interface Settings {
  llm: LLMSettings;
  push: PushSettings;
  sync: SyncSettings;
  theme: string;
  language: string;
  scanConcurrency: number;
  retentionDays: number;
  updatedAt: string;
}

export interface ScanLog {
  id: UUID;
  sourceId: UUID;
  status: ScanStatus;
  postingsFound: number;
  postingsNew: number;
  postingsUpdated: number;
  errorMessage?: string;
  startedAt: string;
  completedAt?: string;
  durationMs?: number;
  rawResponseSample?: string;
}