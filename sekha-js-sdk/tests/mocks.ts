import { Conversation, MemoryConfig } from '../src/types';

export const mockConfig: MemoryConfig = {
  baseURL: 'http://localhost:8080',
  apiKey: 'test-api-key',
  defaultLabel: 'Test',
  timeout: 30000,
};

export const mockConversation: Conversation = {
  id: '123e4567-e89b-12d3-a456-426614174000',
  label: 'Test Conversation',
  folder: '/test',
  messages: [
    { role: 'user', content: 'Hello' },
    { role: 'assistant', content: 'Hi there!' },
  ],
  status: 'active',
  createdAt: '2025-01-01T00:00:00Z',
  updatedAt: '2025-01-01T00:00:00Z',
};

// Helper to create a proper Response mock
export function createMockResponse(data: any, status = 200): Promise<Response> {
  const body = JSON.stringify(data);
  const response: Response = {
    ok: status >= 200 && status < 300,
    status,
    statusText: status === 200 ? 'OK' : 'Error',
    headers: new Headers({ 'Content-Type': 'application/json' }),
    json: async () => data,
    text: async () => body,
    arrayBuffer: async () => new ArrayBuffer(0),
    blob: async () => new Blob(),
    clone: () => response,
    body: null,
    bodyUsed: false,
    formData: async () => new FormData(),
    type: 'basic',
    url: '',
  } as Response;
  
  return Promise.resolve(response);
}

export function createMockErrorResponse(status: number, message: string): Promise<Response> {
  const errorBody = { error: message, code: status };
  const response: Response = {
    ok: false,
    status,
    statusText: message,
    headers: new Headers({ 'Content-Type': 'application/json' }),
    json: async () => errorBody,
    text: async () => JSON.stringify(errorBody),
    arrayBuffer: async () => new ArrayBuffer(0),
    blob: async () => new Blob(),
    clone: () => response,
    body: null,
    bodyUsed: false,
    formData: async () => new FormData(),
    type: 'basic',
    url: '',
  } as Response;
  
  return Promise.resolve(response);
}