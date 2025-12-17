import { jest, describe, it, expect, beforeEach } from '@jest/globals';
import { Commands } from '../src/commands';
import { MemoryController } from '@sekha/sdk';
import { SekhaTreeDataProvider } from '../src/treeView';
import { WebviewProvider } from '../src/webview';
import * as vscode from 'vscode';

// Mock VS Code API
jest.mock('vscode', () => ({
  window: {
    activeTextEditor: undefined,
    showInformationMessage: jest.fn(),
    showWarningMessage: jest.fn(),
    showErrorMessage: jest.fn(),
    showInputBox: jest.fn(),
    showQuickPick: jest.fn(),
    createWebviewPanel: jest.fn(),
  },
  workspace: {
    getConfiguration: jest.fn(),
  },
  ViewColumn: { One: 1 },
}));

describe('Commands', () => {
  let mockMemory: jest.Mocked<MemoryController>;
  let mockTreeView: jest.Mocked<SekhaTreeDataProvider>;
  let mockWebview: jest.Mocked<WebviewProvider>;
  let commands: Commands;

  beforeEach(() => {
    mockMemory = {
      create: jest.fn(),
      getConversation: jest.fn(),
      listConversations: jest.fn(),
      search: jest.fn(),
      assembleContext: jest.fn(),
    } as any;

    mockTreeView = {
      refresh: jest.fn(),
    } as any;

    mockWebview = {
      getConversationHtml: jest.fn().mockReturnValue('<html></html>'),
    } as any;

    commands = new Commands(mockMemory, mockTreeView as any, mockWebview as any);
    jest.clearAllMocks();
  });

  describe('saveConversation', () => {
    it('should show warning when no active editor', async () => {
      vscode.window.activeTextEditor = undefined;

      await commands.saveConversation();

      expect(vscode.window.showWarningMessage).toHaveBeenCalledWith(
        'No active editor to save from'
      );
    });

    it('should save conversation successfully', async () => {
      const mockEditor = {
        document: {
          getText: jest.fn().mockReturnValue('User: Hello\nAssistant: Hi there!'),
        },
      };
      vscode.window.activeTextEditor = mockEditor as any;
      
      vscode.window.showInputBox = jest.fn().mockResolvedValue('Test Conversation');
      mockMemory.create = jest.fn().mockResolvedValue({ id: '123' });

      await commands.saveConversation();

      expect(mockMemory.create).toHaveBeenCalledWith({
        messages: [
          { role: 'user', content: 'Hello' },
          { role: 'assistant', content: 'Hi there!' },
        ],
        label: 'Test Conversation',
        folder: '/vscode',
      });
      expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
        'Conversation saved to Sekha!'
      );
    });
  });

  describe('search', () => {
    it('should handle no query', async () => {
      vscode.window.showInputBox = jest.fn().mockResolvedValue('');

      await commands.search();

      expect(mockMemory.search).not.toHaveBeenCalled();
    });

    it('should search and show results', async () => {
      vscode.window.showInputBox = jest.fn().mockResolvedValue('test query');
      mockMemory.search = jest.fn().mockResolvedValue([
        { id: '123', label: 'Test', score: 0.95, messages: [] },
      ]);
      
      vscode.window.showQuickPick = jest.fn().mockResolvedValue(null);

      await commands.search();

      expect(mockMemory.search).toHaveBeenCalledWith('test query');
      expect(vscode.window.showQuickPick).toHaveBeenCalled();
    });
  });
});