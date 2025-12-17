import * as vscode from 'vscode';
import { MemoryController } from '@sekha/sdk';
import { Conversation } from '@sekha/sdk';

export class SekhaTreeDataProvider implements vscode.TreeDataProvider<SekhaTreeItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<SekhaTreeItem | undefined>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(private memory: MemoryController) {}

  refresh(): void {
    this._onDidChangeTreeData.fire(undefined);
  }

  getTreeItem(element: SekhaTreeItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: SekhaTreeItem): Promise<SekhaTreeItem[]> {
    try {
      if (!element) {
        // Root level: show labels
        return await this.getLabelNodes();
      } else if (element.contextValue === 'label') {
        // Label level: show conversations
        return await this.getConversationNodes(element.label!);
      } else if (element.contextValue === 'conversation') {
        // Conversation level: show messages
        return await this.getMessageNodes(element.conversation!);
      }
      return [];
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to load Sekha tree: ${error}`);
      return [];
    }
  }

  private async getLabelNodes(): Promise<SekhaTreeItem[]> {
    const conversations = await this.memory.listConversations();
    const labels = [...new Set(conversations.map(c => c.label))].sort();
    
    return labels.map(label => new SekhaTreeItem(
      label,
      vscode.TreeItemCollapsibleState.Collapsed,
      'label',
      undefined,
      undefined,
      label
    ));
  }

  private async getConversationNodes(label: string): Promise<SekhaTreeItem[]> {
    const conversations = await this.memory.listConversations({ label });
    const maxConversations = vscode.workspace
      .getConfiguration('sekha')
      .get('maxConversationsInTree', 100);
    
    return conversations
      .slice(0, maxConversations)
      .map(conv => new SekhaTreeItem(
        conv.label,
        vscode.TreeItemCollapsibleState.Collapsed,
        'conversation',
        conv,
        conv.id,
        undefined,
        new vscode.ThemeIcon(conv.status === 'pinned' ? 'pinned' : 'file-text')
      ));
  }

  private async getMessageNodes(conversation: Conversation): Promise<SekhaTreeItem[]> {
    return conversation.messages.map((msg, index) => new SekhaTreeItem(
      `${msg.role}: ${msg.content.substring(0, 50)}...`,
      vscode.TreeItemCollapsibleState.None,
      'message',
      undefined,
      undefined,
      undefined,
      new vscode.ThemeIcon(msg.role === 'user' ? 'account' : 'hubot')
    ));
  }
}

class SekhaTreeItem extends vscode.TreeItem {
  constructor(
    public readonly label: string,
    public readonly collapsibleState: vscode.TreeItemCollapsibleState,
    public readonly contextValue: string,
    public readonly conversation?: Conversation,
    public readonly conversationId?: string,
    public readonly detail?: string,
    public readonly iconPath?: vscode.ThemeIcon
  ) {
    super(label, collapsibleState);
    
    if (contextValue === 'conversation' && conversationId) {
      this.command = {
        command: 'sekha.viewConversation',
        title: 'View Conversation',
        arguments: [conversationId]
      };
    }
  }
}