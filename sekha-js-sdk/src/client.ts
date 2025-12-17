import { MemoryConfig, Conversation, CreateOptions, ListFilter, SearchOptions, 
         ContextOptions, ExportOptions } from './types';
import { SekhaError, SekhaNotFoundError, SekhaValidationError, SekhaAPIError } from './errors';

export class MemoryController {
  private config: MemoryConfig;
  private abortControllers: Map<string, AbortController> = new Map();

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
    
    return this.request(`/api/v1/conversations?${params}`);
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

  async search(query: string, options?: SearchOptions): Promise<any> {
    return this.request('/api/v1/query', {
      method: 'POST',
      body: JSON.stringify({ query, ...options }),
      signal: options?.signal,
    });
  }

  async assembleContext(options: ContextOptions): Promise<any> {
    return this.request('/api/v1/query', {
      method: 'POST',
      body: JSON.stringify(options),
      signal: options.signal,
    });
  }

  async export(options: ExportOptions): Promise<string> {
    const params = new URLSearchParams();
    if (options.label) params.append('label', options.label);
    params.append('format', options.format || 'markdown');
    
    const result = await this.request(`/api/v1/export?${params}`);
    return result.content;
  }

  exportStream(options: ExportOptions): AsyncIterable<string> {
    const stream = new ReadableStream({
      start: async (controller) => {
        try {
          const content = await this.export(options);
          // Simulate streaming by chunking the content
          const chunkSize = 1024;
          for (let i = 0; i < content.length; i += chunkSize) {
            controller.enqueue(content.slice(i, i + chunkSize));
          }
          controller.close();
        } catch (error) {
          controller.error(error);
        }
      },
    });

    return stream.getIterator();
  }

  private async request(endpoint: string, options: RequestInit = {}): Promise<any> {
    const url = `${this.config.baseURL}${endpoint}`;
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
      if (error instanceof SekhaError) throw error;
      throw new SekhaError(`Request failed: ${error.message}`);
    }
  }

  private async handleError(response: Response): Promise<void> {
    const text = await response.text();
    
    switch (response.status) {
      case 400:
        throw new SekhaValidationError('Invalid request', text);
      case 404:
        throw new SekhaNotFoundError('Resource not found');
      case 401:
      case 403:
        throw new SekhaAPIError('Authentication failed', response.status, text);
      default:
        throw new SekhaAPIError(`API error: ${response.status}`, response.status, text);
    }
  }
}