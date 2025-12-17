// Core models
export interface Message {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp?: string;
}

export interface Conversation {
  id: string;
  label: string;
  folder?: string;
  messages: Message[];
  status: 'active' | 'archived' | 'pinned';
  createdAt: string;
  updatedAt: string;
}

export interface MemoryConfig {
  baseURL: string;
  apiKey: string;
  defaultLabel?: string;
  timeout?: number;
}

// API Options
export interface CreateOptions {
  messages: Message[];
  label: string;
  folder?: string;
}

export interface ListFilter {
  label?: string;
  status?: string;
  limit?: number;
  offset?: number;
}

export interface SearchOptions {
  limit?: number;
  labels?: string[];
  signal?: AbortSignal;
}

export interface ContextOptions {
  query: string;
  tokenBudget?: number;
  labels?: string[];
  signal?: AbortSignal;
}

export interface ExportOptions {
  label?: string;
  format?: 'markdown' | 'json';
}

export interface SearchResult {
  id: string;
  label: string;
  messages: Message[];
  score: number;
  status: string;
}

export interface ContextAssembly {
  formattedContext: string;
  estimatedTokens: number;
}