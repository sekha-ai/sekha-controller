import { MemoryConfig, Conversation, CreateOptions, ListFilter, SearchOptions, 
         ContextOptions, ExportOptions, SearchResult, ContextAssembly } from './types';
import { SekhaError, SekhaNotFoundError, SekhaValidationError, SekhaAPIError } from './errors';

export class MemoryController {
  private config: MemoryConfig;

  constructor(config: MemoryConfig) {
    this.config = {
      timeout: 30000,
      ...config,
    };
  }

  async create(options: CreateOptions): Promise<Conversation> {
    return this.request('/api/v1/conversations', {
      method: 'POST',
      body: JSON.stringify(options),
    });
  }

  async getConversation(id: string): Promise<Conversation> {
    return this.request(`/api/v1/conversations/${id}`);
  }

  async listConversations(filter?: ListFilter): Promise<Conversation[]> {
    const params = new URLSearchParams();
    if (filter?.label) params.append('label', filter.label);
    if (filter?.status) params.append('status', filter.status);
    if (filter?.limit) params.append('limit', filter.limit.toString());
    if (filter?.offset) params.append('offset', filter.offset.toString());
    
    const queryString = params.toString();
    const url = queryString ? `/api/v1/conversations?${queryString}` : '/api/v1/conversations';
    
    return this.request(url);
  }

  async updateLabel(id: string, label: string): Promise<void> {
    await this.request(`/api/v1/conversations/${id}/label`, {
      method: 'PUT',
      body: JSON.stringify({ label }),
    });
  }

  async pin(id: string): Promise<void> {
    await this.request(`/api/v1/conversations/${id}/status`, {
      method: 'PUT',
      body: JSON.stringify({ status: 'pinned' }),
    });
  }

  async archive(id: string): Promise<void> {
    await this.request(`/api/v1/conversations/${id}/status`, {
      method: 'PUT',
      body: JSON.stringify({ status: 'archived' }),
    });
  }

  async delete(id: string): Promise<void> {
    await this.request(`/api/v1/conversations/${id}`, {
      method: 'DELETE',
    });
  }

  async search(query: string, options?: SearchOptions): Promise<SearchResult[]> {
    return this.request('/api/v1/query', {
      method: 'POST',
      body: JSON.stringify({ query, ...options }),
      signal: options?.signal,
    });
  }

  async assembleContext(options: ContextOptions): Promise<ContextAssembly> {
    return this.request('/api/v1/query', {
      method: 'POST',
      body: JSON.stringify(options),
      signal: options.signal,
    });
  }

  async export(options: ExportOptions = {}): Promise<string> {
    const params = new URLSearchParams();
    if (options.label) params.append('label', options.label);
    params.append('format', options.format || 'markdown');
    
    const result = await this.request(`/api/v1/export?${params.toString()}`);
    return result.content;
  }

  exportStream(options: ExportOptions): AsyncIterable<string> {
  const self = this;
  
  return {
    [Symbol.asyncIterator]: async function* () {
      const content = await self.export(options);
      const chunkSize = 1024;
      
      for (let i = 0; i < content.length; i += chunkSize) {
        yield content.slice(i, Math.min(i + chunkSize, content.length));
      }
    }
  };
}

  private chunkString(str: string, size: number): string[] {
    const chunks: string[] = [];
    for (let i = 0; i < str.length; i += size) {
      chunks.push(str.slice(i, i + size));
    }
    return chunks;
  }

  private async request(endpoint: string, options: RequestInit = {}): Promise<any> {
    const url = `${this.config.baseURL}${endpoint}`;
    
    // Create abort controller with timeout
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);
    
    try {
      const response = await fetch(url, {
        ...options,
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.config.apiKey}`,
          ...options.headers,
        },
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        await this.handleError(response);
      }

      const text = await response.text();
      return text ? JSON.parse(text) : null;
    } catch (error) {
      clearTimeout(timeoutId);
      
      // Check if it's an abort error (timeout)
      if (error instanceof Error && error.name === 'AbortError') {
        throw new SekhaError(`Request timed out after ${this.config.timeout}ms`);
      }
      
      if (error instanceof SekhaError) throw error;
      throw new SekhaError(`Request failed: ${error}`);
    }
  }

  private async handleError(response: Response): Promise<void> {
    const text = await response.text();
    const errorData = text ? JSON.parse(text) : { error: 'Unknown error' };
    
    switch (response.status) {
      case 400:
        throw new SekhaValidationError(errorData.error || 'Invalid request', text);
      case 404:
        throw new SekhaNotFoundError(errorData.error || 'Not found');
      case 401:
      case 403:
        throw new SekhaAPIError('Authentication failed', response.status, text);
      default:
        throw new SekhaAPIError(
          errorData.error || `API error: ${response.status}`, 
          response.status, 
          text
        );
    }
  }
}