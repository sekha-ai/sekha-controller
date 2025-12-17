# Sekha VS Code Extension

Connect your VS Code to Sekha AI Memory Controller.

## Features

- **Activity Bar**: Browse conversations by label
- **Commands**: Save chats, search memory, insert context
- **Auto-save**: Automatically save conversations every 5 minutes

## Installation

1. Build: `npm run package`
2. Install in VS Code: Extension sidebar → Install from VSIX

## Configuration

```json
{
  "sekha.apiUrl": "http://localhost:8080",
  "sekha.apiKey": "sk-...",
  "sekha.autoSave": true,
  "sekha.autoSaveInterval": 5
}

Commands
Sekha: Save this conversation - Save current chat
Sekha: Search memory - Fuzzy search all conversations
Sekha: Insert context into chat - Add context to current chat
Sekha: Refresh tree view - Reload conversation tree

Usage
Set API URL and key in settings
Use commands from command palette (Ctrl+Shift+P)
Browse conversations in Activity Bar
Click to view conversation details


Create `sekha-vscode/src/extension.ts`:

```typescript
import * as vscode from 'vscode';
import { MemoryController, MemoryConfig } from '@sekha/sdk';
import { SekhaTreeDataProvider } from './treeView';
import { SekhaCommands } from './commands';

export function activate(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration('sekha');
  
  const memory = new MemoryController({
    baseURL: config.get('apiUrl', 'http://localhost:8080'),
    apiKey: config.get('apiKey', ''),
  });

  // Tree view
  const treeDataProvider = new SekhaTreeDataProvider(memory);
  vscode.window.registerTreeDataProvider('sekhaExplorer', treeDataProvider);

  // Commands
  const commands = new SekhaCommands(memory, treeDataProvider);
  
  context.subscriptions.push(
    vscode.commands.registerCommand('sekha.saveConversation', 
      () => commands.saveConversation()),
    vscode.commands.registerCommand('sekha.search', 
      () => commands.search()),
    vscode.commands.registerCommand('sekha.insertContext', 
      () => commands.insertContext()),
    vscode.commands.registerCommand('sekha.refresh', 
      () => treeDataProvider.refresh()),
    vscode.commands.registerCommand('sekha.viewConversation', 
      (id: string) => commands.viewConversation(id))
  );

  // Auto-save
  if (config.get('autoSave', false)) {
    const interval = config.get('autoSaveInterval', 5) * 60000;
    setInterval(() => commands.saveConversation(), interval);
  }
}

export function deactivate() {}