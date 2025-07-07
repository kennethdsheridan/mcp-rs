# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MCP-RS is a Model Context Protocol CLI tool that provides unified access to multiple API resources (Notion, Linear) using hexagonal architecture (ports and adapters pattern).

## Development Commands

### Building and Running
```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run the CLI with help
cargo run -- --help

# Run with specific commands
cargo run -- fetch --source notion
cargo run -- search "test query"
```

### Environment Setup
Required environment variables for API access:
```bash
export NOTION_API_KEY="your_notion_integration_token"
export LINEAR_API_KEY="your_linear_api_key"
```

## Architecture

This codebase follows **Hexagonal Architecture (Ports and Adapters)**:

### Core Layers
- **Domain** (`src/domain/`): Contains core entities (`Resource`, `Query`) and business rules. The `Resource` struct is the central entity representing data from any provider.
- **Ports** (`src/ports/`): Defines interfaces, primarily `ResourceProvider` trait that all API adapters must implement.
- **Application** (`src/application/`): Contains `ResourceService` which orchestrates between ports and coordinates multiple providers.
- **Infrastructure** (`src/infrastructure/`): External concerns including API adapters and CLI.

### Key Architecture Patterns

**Provider Pattern**: Each API integration (Notion, Linear) implements the `ResourceProvider` trait with these methods:
- `fetch_resources()` - Query-based resource retrieval
- `fetch_resource_by_id()` - Single resource by ID
- `search()` - Full-text search
- `provider_name()` - Provider identification

**Resource Abstraction**: All providers return unified `Resource` objects with standardized fields (`id`, `title`, `content`, `metadata`, `source`). Provider-specific data goes into the `metadata` HashMap.

**Error Handling**: Uses `DomainError` enum with `thiserror` for structured error handling across all layers.

## Adding New Providers

1. Create new module in `src/infrastructure/adapters/your_provider/`
2. Implement `ResourceProvider` trait
3. Add provider initialization in `src/main.rs` (look for the provider configuration section)
4. Update `QuerySource` enum in `src/domain/mod.rs` if needed
5. Update CLI source parsing in `src/infrastructure/cli/mod.rs`

## Resource ID Conventions

Resources use prefixed IDs to identify their source:
- Notion: `notion_{page_id}`
- Linear: `linear_{issue_id}`

This enables automatic provider detection in `ResourceService::fetch_resource_by_id()`.

## CLI Commands Structure

The CLI uses `clap` with these main commands:
- `fetch` - Retrieve resources with optional filters
- `get` - Get specific resource by ID  
- `search` - Full-text search across providers
- `providers` - List configured providers
- `config` - Manage configuration and test connections

For Notion queries, the `database_id` filter is required for the `fetch` command.