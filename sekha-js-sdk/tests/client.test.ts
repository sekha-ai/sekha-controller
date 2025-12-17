import { jest, describe, it, expect, beforeEach } from '@jest/globals';
import { MemoryController } from '../src/client';
import { mockConfig, mockConversation, createMockResponse, createMockErrorResponse } from './mocks';
import { SekhaNotFoundError, SekhaValidationError, SekhaAPIError } from '../src/errors';

// Properly typed mock fetch
const mockFetch = jest.fn() as jest.MockedFunction<typeof fetch>;
(global.fetch as any) = mockFetch;

describe('MemoryController', () => {
  let client: MemoryController;

  beforeEach(() => {
    client = new MemoryController(mockConfig);
    mockFetch.mockClear();
  });

  describe('constructor', () => {
    it('should initialize with config', () => {
      expect(client).toBeDefined();
      expect((client as any).config.baseURL).toBe(mockConfig.baseURL);
      expect((client as any).config.timeout).toBe(30000);
    });

    it('should override default timeout', () => {
      const customClient = new MemoryController({ ...mockConfig, timeout: 5000 });
      expect((customClient as any).config.timeout).toBe(5000);
    });
  });

  describe('create', () => {
    it('should create a conversation', async () => {
      mockFetch.mockResolvedValue(await createMockResponse(mockConversation, 201));

      const result = await client.create({
        messages: mockConversation.messages,
        label: mockConversation.label,
      });

      expect(result).toEqual(mockConversation);
      expect(mockFetch).toHaveBeenCalled();
    });

    it('should handle validation error', async () => {
      mockFetch.mockResolvedValue(await createMockErrorResponse(400, 'Invalid conversation data'));

      await expect(client.create({ messages: [], label: '' }))
        .rejects.toThrow(SekhaValidationError);
    });
  });

  describe('getConversation', () => {
    it('should retrieve a conversation', async () => {
      mockFetch.mockResolvedValue(await createMockResponse(mockConversation));

      const result = await client.getConversation(mockConversation.id);

      expect(result).toEqual(mockConversation);
      expect(mockFetch).toHaveBeenCalled();
    });

    it('should throw not found error', async () => {
      mockFetch.mockResolvedValue(await createMockErrorResponse(404, 'Not found'));

      await expect(client.getConversation('invalid-id'))
        .rejects.toThrow(SekhaNotFoundError);
    });
  });

  describe('listConversations', () => {
    it('should list all conversations', async () => {
      mockFetch.mockResolvedValue(await createMockResponse([mockConversation]));

      const results = await client.listConversations();

      expect(results).toHaveLength(1);
      expect(results[0]).toEqual(mockConversation);
    });

    it('should apply filters', async () => {
      mockFetch.mockResolvedValue(await createMockResponse([mockConversation]));

      await client.listConversations({ label: 'Test', status: 'active' });

      expect(mockFetch).toHaveBeenCalled();
      const callUrl = mockFetch.mock.calls[0][0] as string;
      expect(callUrl).toContain('label=Test');
    });
  });

  describe('updateLabel', () => {
    it('should update conversation label', async () => {
      mockFetch.mockResolvedValue(await createMockResponse({}, 200));

      await client.updateLabel(mockConversation.id, 'New Label');

      expect(mockFetch).toHaveBeenCalled();
    });
  });

  describe('pin', () => {
    it('should pin a conversation', async () => {
      mockFetch.mockResolvedValue(await createMockResponse({}, 200));

      await client.pin(mockConversation.id);

      expect(mockFetch).toHaveBeenCalled();
    });
  });

  describe('archive', () => {
    it('should archive a conversation', async () => {
      mockFetch.mockResolvedValue(await createMockResponse({}, 200));

      await client.archive(mockConversation.id);

      expect(mockFetch).toHaveBeenCalled();
    });
  });

  describe('delete', () => {
    it('should delete a conversation', async () => {
      mockFetch.mockResolvedValue(await createMockResponse({}, 200));

      await client.delete(mockConversation.id);

      expect(mockFetch).toHaveBeenCalled();
    });
  });

  describe('search', () => {
    it('should perform semantic search', async () => {
      mockFetch.mockResolvedValue(await createMockResponse([{ ...mockConversation, score: 0.95 }]));

      const results = await client.search('test query');

      expect(results).toHaveLength(1);
      expect(results[0].score).toBe(0.95);
    });
  });

  describe('assembleContext', () => {
    it('should assemble context for LLM', async () => {
      const mockContext = {
        formattedContext: "Previous conversation...",
        estimatedTokens: 1500,
      };
      
      mockFetch.mockResolvedValue(await createMockResponse(mockContext));

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
      
      mockFetch.mockResolvedValue(await createMockResponse(mockExport));

      const result = await client.export({ label: 'Test', format: 'markdown' });

      expect(result).toBe(mockExport.content);
    });

    it('should export as JSON', async () => {
      const mockExport = {
        content: '[{"id": "123"}]',
        format: 'json',
        conversationCount: 1,
      };
      
      mockFetch.mockResolvedValue(await createMockResponse(mockExport));

      const result = await client.export({ format: 'json' });

      expect(result).toBe(mockExport.content);
    });

    it('should handle invalid format error', async () => {
      mockFetch.mockResolvedValue(await createMockErrorResponse(400, 'Invalid format'));

      await expect(client.export({ format: 'invalid' as any }))
        .rejects.toThrow(SekhaValidationError);
    });
  });

  describe('exportStream', () => {
    it('should stream export content', async () => {
      const mockExport = {
        content: 'A'.repeat(5000),
        format: 'markdown',
        conversationCount: 1,
      };
      
      mockFetch.mockResolvedValue(await createMockResponse(mockExport));

      const stream = client.exportStream({ format: 'markdown' });
      const chunks: string[] = [];

      for await (const chunk of stream) {
        chunks.push(chunk);
      }

      expect(chunks.length).toBeGreaterThan(1);
      expect(chunks.join('')).toBe(mockExport.content);
    });
  });

  describe('timeout handling', () => {
    it('should have correct timeout configuration', () => {
      const customClient = new MemoryController({ ...mockConfig, timeout: 5000 });
      expect((customClient as any).config.timeout).toBe(5000);
    });
  });

  describe('error handling', () => {
    it('should handle network errors', async () => {
      mockFetch.mockRejectedValue(new Error('Network error'));

      await expect(client.getConversation('123'))
        .rejects.toThrow();

      try {
        await client.getConversation('123');
      } catch (error) {
        expect((error as Error).message).toContain('Network error');
      }
    });

    it('should handle API errors with status codes', async () => {
      mockFetch.mockResolvedValue(await createMockErrorResponse(500, 'Internal server error'));

      await expect(client.getConversation('123'))
        .rejects.toThrow(SekhaAPIError);
    });
  });
});