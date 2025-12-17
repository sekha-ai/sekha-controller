import { jest, describe, it, expect, beforeEach } from '@jest/globals';
import { MemoryController } from '../src/client';
import { mockConfig, mockConversation, createMockResponse, createMockErrorResponse } from './mocks';
import { SekhaNotFoundError, SekhaValidationError, SekhaAPIError } from '../src/errors';

describe('MemoryController', () => {
  let client: MemoryController;

  beforeEach(() => {
    client = new MemoryController(mockConfig);
    jest.clearAllMocks();
  });

  describe('constructor', () => {
    it('should initialize with config', () => {
      expect(client).toBeDefined();
      expect(client['config'].baseURL).toBe(mockConfig.baseURL);
      expect(client['config'].timeout).toBe(30000); // default
    });

    it('should override default timeout', () => {
      const customClient = new MemoryController({ ...mockConfig, timeout: 5000 });
      expect(customClient['config'].timeout).toBe(5000);
    });
  });

  describe('create', () => {
    it('should create a conversation', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse(mockConversation, 201)
      );

      const result = await client.create({
        messages: mockConversation.messages,
        label: mockConversation.label,
      });

      expect(result).toEqual(mockConversation);
      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/v1/conversations',
        expect.objectContaining({
          method: 'POST',
          body: expect.stringContaining('Test Conversation'),
        })
      );
    });

    it('should handle validation error', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockErrorResponse(400, 'Invalid conversation data')
      );

      await expect(client.create({ messages: [], label: '' }))
        .rejects.toThrow(SekhaValidationError);
    });
  });

  describe('getConversation', () => {
    it('should retrieve a conversation', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse(mockConversation)
      );

      const result = await client.getConversation(mockConversation.id);

      expect(result).toEqual(mockConversation);
      expect(global.fetch).toHaveBeenCalledWith(
        `http://localhost:8080/api/v1/conversations/${mockConversation.id}`,
        expect.any(Object)
      );
    });

    it('should throw not found error', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockErrorResponse(404, 'Not found')
      );

      await expect(client.getConversation('invalid-id'))
        .rejects.toThrow(SekhaNotFoundError);
    });
  });

  describe('listConversations', () => {
    it('should list all conversations', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse([mockConversation])
      );

      const results = await client.listConversations();

      expect(results).toHaveLength(1);
      expect(results[0]).toEqual(mockConversation);
    });

    it('should apply filters', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse([mockConversation])
      );

      await client.listConversations({ label: 'Test', status: 'active' });

      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('label=Test'),
        expect.any(Object)
      );
    });
  });

  describe('updateLabel', () => {
    it('should update conversation label', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse({}, 200)
      );

      await client.updateLabel(mockConversation.id, 'New Label');

      expect(global.fetch).toHaveBeenCalledWith(
        `http://localhost:8080/api/v1/conversations/${mockConversation.id}/label`,
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ label: 'New Label' }),
        })
      );
    });
  });

  describe('pin', () => {
    it('should pin a conversation', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse({}, 200)
      );

      await client.pin(mockConversation.id);

      expect(global.fetch).toHaveBeenCalledWith(
        `http://localhost:8080/api/v1/conversations/${mockConversation.id}/status`,
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ status: 'pinned' }),
        })
      );
    });
  });

  describe('archive', () => {
    it('should archive a conversation', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse({}, 200)
      );

      await client.archive(mockConversation.id);

      expect(global.fetch).toHaveBeenCalledWith(
        `http://localhost:8080/api/v1/conversations/${mockConversation.id}/status`,
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ status: 'archived' }),
        })
      );
    });
  });

  describe('delete', () => {
    it('should delete a conversation', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse({}, 200)
      );

      await client.delete(mockConversation.id);

      expect(global.fetch).toHaveBeenCalledWith(
        `http://localhost:8080/api/v1/conversations/${mockConversation.id}`,
        expect.objectContaining({ method: 'DELETE' })
      );
    });
  });

  describe('search', () => {
    it('should perform semantic search', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse([{ ...mockConversation, score: 0.95 }])
      );

      const results = await client.search('test query');

      expect(results).toHaveLength(1);
      expect(results[0].score).toBe(0.95);
    });

    it('should pass signal for cancellation', async () => {
      const controller = new AbortController();
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse([])
      );

      await client.search('query', { signal: controller.signal });

      expect(global.fetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({ signal: controller.signal })
      );
    });
  });

  describe('assembleContext', () => {
    it('should assemble context for LLM', async () => {
      const mockContext = {
        formattedContext: "Previous conversation...",
        estimatedTokens: 1500,
      };
      
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse(mockContext)
      );

      const result = await client.assembleContext({
        query: 'test',
        tokenBudget: 8000,
      });

      expect(result.formattedContext).toBe(mockContext.formattedContext);
      expect(result.estimatedTokens).toBe(1500);
    });
  });

  describe('export', () => {
    it('should export as markdown', async () => {
      const mockExport = {
        content: '# Export\n\n## Conversation',
        format: 'markdown',
        conversationCount: 1,
      };
      
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse(mockExport)
      );

      const result = await client.export({ label: 'Test', format: 'markdown' });

      expect(result).toBe(mockExport.content);
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('format=markdown'),
        expect.any(Object)
      );
    });

    it('should export as JSON', async () => {
      const mockExport = {
        content: '[{"id": "123"}]',
        format: 'json',
        conversationCount: 1,
      };
      
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse(mockExport)
      );

      const result = await client.export({ format: 'json' });

      expect(result).toBe(mockExport.content);
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('format=json'),
        expect.any(Object)
      );
    });

    it('should handle invalid format error', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockErrorResponse(400, 'Invalid format')
      );

      await expect(client.export({ format: 'invalid' as any }))
        .rejects.toThrow(SekhaValidationError);
    });
  });

  describe('exportStream', () => {
    it('should stream export content', async () => {
      const mockExport = {
        content: 'A'.repeat(5000), // Large content
        format: 'markdown',
        conversationCount: 1,
      };
      
      global.fetch = jest.fn().mockResolvedValue(
        createMockResponse(mockExport)
      );

      const stream = client.exportStream({ format: 'markdown' });
      const chunks: string[] = [];

      for await (const chunk of stream) {
        chunks.push(chunk);
      }

      expect(chunks.length).toBeGreaterThan(1); // Should be chunked
      expect(chunks.join('')).toBe(mockExport.content);
    });
  });

  describe('timeout handling', () => {
    it('should abort request after timeout', async () => {
      const customClient = new MemoryController({ ...mockConfig, timeout: 100 });
      
      // Mock fetch that never resolves
      global.fetch = jest.fn().mockImplementation(
        () => new Promise(() => {}) // Hang forever
      );

      await expect(customClient.getConversation('123'))
        .rejects.toThrow('aborted');

      // Verify AbortController was used
      expect(global.fetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          signal: expect.any(AbortSignal),
        })
      );
    });
  });

  describe('error handling', () => {
    it('should handle network errors', async () => {
      global.fetch = jest.fn().mockRejectedValue(
        new Error('Network error')
      );

      await expect(client.getConversation('123'))
        .rejects.toThrow('Request failed: Network error');
    });

    it('should handle API errors with status codes', async () => {
      global.fetch = jest.fn().mockResolvedValue(
        createMockErrorResponse(500, 'Internal server error')
      );

      await expect(client.getConversation('123'))
        .rejects.toThrow(SekhaAPIError);
    });
  });
});