import * as vscode from 'vscode';
import { Conversation } from '@sekha/sdk';

export class WebviewProvider {
  constructor(private extensionUri: vscode.Uri) {}

  getConversationHtml(conversation: Conversation): string {
    const messagesHtml = conversation.messages
      .map(msg => `
        <div class="message ${msg.role}">
          <div class="message-header">${msg.role.toUpperCase()}</div>
          <div class="message-content">${this.escapeHtml(msg.content)}</div>
        </div>
      `)
      .join('');

    return `
      <!DOCTYPE html>
      <html>
      <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>${this.escapeHtml(conversation.label)}</title>
        <style>
          body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            margin: 0;
            padding: 20px;
            background: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
          }
          .conversation-header {
            margin-bottom: 20px;
            padding-bottom: 10px;
            border-bottom: 1px solid var(--vscode-panel-border);
          }
          .conversation-title {
            font-size: 24px;
            font-weight: 600;
            margin: 0 0 5px 0;
          }
          .conversation-meta {
            font-size: 12px;
            color: var(--vscode-descriptionForeground);
          }
          .message {
            margin-bottom: 15px;
            padding: 12px;
            border-radius: 6px;
            background: var(--vscode-textCodeBlock-background);
          }
          .message.user {
            background: var(--vscode-textBlockQuote-background);
          }
          .message-header {
            font-size: 12px;
            font-weight: 600;
            text-transform: uppercase;
            margin-bottom: 8px;
            color: var(--vscode-descriptionForeground);
          }
          .message-content {
            white-space: pre-wrap;
            line-height: 1.5;
          }
          .message.assistant .message-content {
            font-family: var(--vscode-editor-font-family);
          }
        </style>
      </head>
      <body>
        <div class="conversation-header">
          <h1 class="conversation-title">${this.escapeHtml(conversation.label)}</h1>
          <div class="conversation-meta">
            ID: ${conversation.id} | 
            Folder: ${conversation.folder} | 
            Status: ${conversation.status} | 
            Created: ${new Date(conversation.createdAt).toLocaleString()}
          </div>
        </div>
        <div class="messages">
          ${messagesHtml}
        </div>
      </body>
      </html>
    `;
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
}