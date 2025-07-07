# MCP-RS

A Model Context Protocol CLI tool built in Rust that provides unified access to multiple API resources including Notion and Linear.

## Architecture

This project uses the **Ports and Adapters (Hexagonal Architecture)** pattern:

- **Domain**: Core business logic and entities (`src/domain/`)
- **Ports**: Interfaces for external dependencies (`src/ports/`)
- **Application**: Use cases and application services (`src/application/`)
- **Infrastructure**: External adapters and implementations (`src/infrastructure/`)
  - **Adapters**: API integrations (Notion, Linear)
  - **CLI**: Command-line interface

## Features

- **Multi-provider support**: Notion and Linear APIs
- **Unified resource model**: Consistent interface across providers
- **Search capabilities**: Full-text search across all resources
- **CLI interface**: Easy to use command-line tool
- **Extensible**: Easy to add new providers

## Setup

1. **Set up API credentials**:
   ```bash
   export NOTION_API_KEY="your_notion_integration_token"
   export LINEAR_API_KEY="your_linear_api_key"
   ```

2. **For Notion**: 
   - Create an integration at https://www.notion.so/my-integrations
   - Share your databases with the integration
   - Copy the integration token

3. **For Linear**:
   - Go to Linear Settings > API
   - Create a new API key
   - Copy the API key

## Usage

### Fetch resources
```bash
# Fetch from all providers
mcp-rs fetch

# Fetch from specific provider
mcp-rs fetch --source notion
mcp-rs fetch --source linear

# Fetch with filters (for Notion, requires database_id)
mcp-rs fetch --source notion --filter database_id=your_database_id

# Limit results
mcp-rs fetch --limit 10
```

### Get specific resource
```bash
mcp-rs get notion_page_id
mcp-rs get linear_issue_id
```

### Search resources
```bash
# Search all providers
mcp-rs search "project requirements"

# Search specific providers
mcp-rs search "bug fix" --source linear
mcp-rs search "meeting notes" --source notion

# Limit search results
mcp-rs search "documentation" --limit 5
```

### Provider management
```bash
# List configured providers
mcp-rs providers

# Check configuration
mcp-rs config list

# Test connections
mcp-rs config test
mcp-rs config test notion
```

## Development

### Building
```bash
cargo build --release
```

### Running
```bash
cargo run -- --help
```

### Adding a new provider

1. Create adapter in `src/infrastructure/adapters/your_provider/`
2. Implement the `ResourceProvider` trait
3. Add provider to `src/main.rs` configuration
4. Update the `QuerySource` enum if needed