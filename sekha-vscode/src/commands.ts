import * as vscode from 'vscode';
import { MemoryController } from '../../sekha-js-sdk/src/client';
import { SekhaTreeDataProvider } from './treeView';
import { WebviewProvider } from './webview';

export class Commands {
  constructor(
    private memory: MemoryController,
    private treeView: SekhaTreeDataProvider,
    private webview: WebviewProvider
  ) {}

  async saveConversation(): Promise<void> {
    try {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showWarningMessage('No active editor to save from');
        return;
      }

      const content = editor.document.getText();
      if (!content.trim()) {
        vscode.window.showWarningMessage('Editor is empty');
        return;
      }

      // Parse conversation from editor content
      const messages = this.parseMessages(content);
      
      const label = await vscode.window.showInputBox({
        prompt: 'Enter conversation label',
        value: 'Saved from VS Code',
      });

      if (!label) return;

      await this.memory.create({
        messages,
        label,
        folder: '/vscode',
      });

      vscode.window.showInformationMessage('Conversation saved to Sekha!');
      this.treeView.refresh();

    } catch (error) {
      vscode.window.showErrorMessage(`Failed to save conversation: ${error}`);
    }
  }

  async autoSaveConversation(): Promise<void> {
    // Similar to saveConversation but silent
    try {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;

      const content = editor.document.getText();
      if (!content.trim()) return;

      const messages = this.parseMessages(content);
      if (messages.length === 0) return;

      await this.memory.create({
        messages,
        label: 'Auto-saved from VS Code',
        folder: '/vscode/auto',
      });

    } catch (error) {
      console.error('Auto-save failed:', error);
    }
  }

  async search(): Promise<void> {
    try {
      const query = await vscode.window.showInputBox({
        prompt: 'Search Sekha memory',
        placeHolder: 'Enter search query...',
      });

      if (!query) return;

      const results = await this.memory.search(query);
      
      if (results.length === 0) {
        vscode.window.showInformationMessage('No results found');
        return;
      }

      const items = results.map(r => ({
        label: r.label || 'Untitled',
        description: `Score: ${(r.score * 100).toFixed(1)}%`,
        detail: r.messages?.[0]?.content?.substring(0, 100),
        conversation: r,
      }));

      const selected = await vscode.window.showQuickPick(items, {
        placeHolder: 'Select conversation to view',
      });

      if (selected?.conversation?.id) {
        await this.viewConversation(selected.conversation.id);
      }

    } catch (error) {
      vscode.window.showErrorMessage(`Search failed: ${error}`);
    }
  }

  async insertContext(): Promise<void> {
    try {
      const query = await vscode.window.showInputBox({
        prompt: 'What context do you need?',
        placeHolder: 'Enter query for context assembly...',
      });

      if (!query) return;

      const context = await this.memory.assembleContext({
        query,
        tokenBudget: 4000, // Reasonable limit for VS Code
      });

      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showWarningMessage('No active editor to insert into');
        return;
      }

      await editor.edit(edit => {
        const position = editor.selection.active;
        edit.insert(position, `\n\n### Context from Sekha\n\n${context.formattedContext}\n\n`);
      });

      vscode.window.showInformationMessage('Context inserted from Sekha!');

    } catch (error) {
      vscode.window.showErrorMessage(`Failed to insert context: ${error}`);
    }
  }

  async viewConversation(id: string): Promise<void> {
    try {
      const conversation = await this.memory.getConversation(id);
      
      // Create or show webview panel
      const panel = vscode.window.createWebviewPanel(
        'sekhaConversation',
        conversation.label || 'Conversation',
        vscode.ViewColumn.One,
        { enableScripts: true }
      );

      panel.webview.html = this.webview.getConversationHtml(conversation);

    } catch (error) {
      vscode.window.showErrorMessage(`Failed to load conversation: ${error}`);
    }
  }

  private parseMessages(content: string): Array<{ role: string; content: string }> {
    // Simple parser - can be enhanced
    const messages: Array<{ role: string; content: string }> = [];
    const lines = content.split('\n');
    
    let currentRole: string | null = null;
    let currentContent: string[] = [];
    
    for (const line of lines) {
      const trimmed = line.trim();
      
      if (trimmed.startsWith('User:') || trimmed.startsWith('Assistant:')) {
        if (currentRole && currentContent.length > 0) {
          messages.push({
            role: currentRole,
            content: currentContent.join('\n').trim(),
          });
        }
        
        currentRole = trimmed.startsWith('User:') ? 'user' : 'assistant';
        currentContent = [trimmed.replace(/^(User|Assistant):\s*/, '')];
      } else if (trimmed && currentRole) {
        currentContent.push(line);
      }
    }
    
    if (currentRole && currentContent.length > 0) {
      messages.push({
        role: currentRole,
        content: currentContent.join('\n').trim(),
      });
    }
    
    return messages;
  }
}