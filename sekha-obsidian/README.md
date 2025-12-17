# Sekha Obsidian Plugin

Connect your Sekha AI memory to your Obsidian knowledge base.

## Features

- Import Sekha conversations as Obsidian notes
- Search Sekha from Obsidian command palette
- Embed context using code blocks
- Wiki-style links to conversations

## Setup

1. Copy to `.obsidian/plugins/`
2. Enable in Community plugins
3. Configure API URL and key

## Embed Syntax

```sekha
{
  "label": "Project:AI",
  "limit": 5,
  "format": "summary"
}

Wiki Links
Type [[sekha: and select a conversation to link.
Commands:
"Sekha: Import conversation" - Import single conversation
"Sekha: Import by label" - Bulk import by label
"Sekha: Search memory" - Search and link
