# @sekha/sdk

JavaScript/TypeScript SDK for Sekha AI Memory Controller.

## Installation

```bash
npm install @sekha/sdk
# or
yarn add @sekha/sdk
# or
pnpm add @sekha/sdk


### Usage
# Basic Configuration

import { MemoryController, MemoryConfig } from '@sekha/sdk';

const config: MemoryConfig = {
  baseURL: 'http://localhost:8080',
  apiKey: process.env.SEKHA_API_KEY,
  defaultLabel: 'Work',
  timeout: 30000,
};

const memory = new MemoryController(config);

## Create and Store Conversations

const conversation = await memory.create({
  messages: [
    { role: 'user', content: 'How do we solve token limits?' },
    { role: 'assistant', content: 'Use a memory controller...' }
  ],
  label: 'Project:AI-Memory',
  folder: 'Work/2025'
});

console.log(`Created conversation: ${conversation.id}`);

## Retrieve Context for LLM

const context = await memory.assembleContext({
  query: 'token limits solution',
  tokenBudget: 8000,
  labels: ['Project:AI-Memory']
});

// context.formattedContext contains the retrieved conversations
// context.estimatedTokens contains token count

## Streaming Exports

// Stream large exports to avoid memory issues
const stream = memory.exportStream({
  label: 'Project:AI-Memory',
  format: 'markdown'
});

for await (const chunk of stream) {
  process.stdout.write(chunk);
}

## AbortController Support

const controller = new AbortController();
const timeout = setTimeout(() => controller.abort(), 5000);

try {
  const result = await memory.search('authentication', {
    signal: controller.signal
  });
  clearTimeout(timeout);
} catch (error) {
  if (error.name === 'AbortError') {
    console.log('Request was aborted');
  }
}

## Error Handling

import { SekhaError, SekhaNotFoundError, SekhaValidationError } from '@sekha/sdk';

try {
  await memory.getConversation('invalid-id');
} catch (error) {
  if (error instanceof SekhaNotFoundError) {
    console.log('Conversation not found');
  } else if (error instanceof SekhaError) {
    console.log('Sekha API error:', error.message);
  }
}


### API Reference
# MemoryController
create(options: CreateOptions): Promise<Conversation>
getConversation(id: string): Promise<Conversation>
listConversations(filter?: ListFilter): Promise<Conversation[]>
updateLabel(id: string, label: string): Promise<void>
pin(id: string): Promise<void>
archive(id: string): Promise<void>
delete(id: string): Promise<void>
search(query: string, options?: SearchOptions): Promise<SearchResult[]>
assembleContext(options: ContextOptions): Promise<ContextAssembly>
export(options: ExportOptions): Promise<string>
exportStream(options: ExportOptions): AsyncIterable<string>

### License
AGPL-3.0

