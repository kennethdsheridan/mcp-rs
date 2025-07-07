# MCP-RS Quickstart Guide

This guide will get you up and running with MCP-RS in minutes using Nix and Doppler for the best developer experience.

## Prerequisites

- A Unix-like system (Linux, macOS, or WSL on Windows)
- Internet connection for downloading dependencies
- API access to Notion and/or Linear

## Step 1: Install Determinate Nix

Determinate Nix provides a streamlined, production-ready Nix installation with enhanced features and better performance.

```bash
# Install Determinate Nix (this may take a few minutes)
curl -fsSL https://install.determinate.systems/nix | sh -s -- install --determinate

# Follow the instructions to reload your shell or run:
. /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh
```

**What this does:**
- Installs Nix package manager with flakes support enabled
- Sets up the Nix daemon for multi-user operation
- Configures binary caches for faster package downloads
- Enables experimental features like flakes and the new `nix` command

## Step 2: Clone and Enter Development Environment

```bash
# Clone the repository
git clone https://github.com/your-username/mcp-rs.git
cd mcp-rs

# Enter the Nix development environment
# This will automatically download and set up all dependencies
nix develop
```

**What this does:**
- Downloads and installs the Rust toolchain (specific version with all components)
- Installs Doppler CLI for secure secret management
- Sets up all development tools (cargo-watch, clippy, rustfmt, etc.)
- Configures SSL certificates and environment variables
- Creates a `.env` template file for your API keys

## Step 3: Set Up API Credentials

### Option A: Using Doppler (Recommended for Production)

Doppler provides secure, centralized secret management with audit trails and team collaboration features.

```bash
# Set up Doppler project (follow the interactive prompts)
doppler setup

# Add your API keys securely
doppler secrets set NOTION_API_KEY=your_notion_integration_token_here
doppler secrets set LINEAR_API_KEY=your_linear_api_key_here

# Verify secrets are set
doppler secrets list
```

### Option B: Using Environment File (Development)

```bash
# The .env file was created automatically when you ran 'nix develop'
# Edit it with your actual API keys
nano .env

# Or copy it to a local version
cp .env .env.local
nano .env.local
```

## Step 4: Obtain API Keys

### Notion API Key

1. Go to [https://www.notion.so/my-integrations](https://www.notion.so/my-integrations)
2. Click "New integration"
3. Give it a name (e.g., "MCP-RS CLI")
4. Select the workspace you want to access
5. Copy the "Internal Integration Token"
6. **Important**: Share your databases with this integration:
   - Go to your Notion database
   - Click the "..." menu → "Add connections"
   - Select your integration

### Linear API Key

1. Go to your Linear workspace settings
2. Navigate to "API" section
3. Click "Create new API key"
4. Give it a name (e.g., "MCP-RS CLI")
5. Copy the generated API key

## Step 5: Build and Test

```bash
# Build the project (this verifies everything is set up correctly)
cargo build

# Run tests to ensure everything works
cargo test

# Test the CLI with help
cargo run -- --help
```

## Step 6: Verify API Connections

```bash
# Test both APIs using Doppler
doppler run -- cargo run -- config test

# Or test individual providers
doppler run -- cargo run -- config test notion
doppler run -- cargo run -- config test linear

# List configured providers
doppler run -- cargo run -- providers
```

## Step 7: Try Basic Operations

```bash
# Fetch resources from all providers
doppler run -- cargo run -- fetch

# Fetch from specific provider with database filter (for Notion)
doppler run -- cargo run -- fetch --source notion --filter database_id=your_database_id

# Search across all providers
doppler run -- cargo run -- search "project requirements"

# Get a specific resource by ID
doppler run -- cargo run -- get notion_page_id_here
```

## Development Workflow

### Daily Development

```bash
# Enter development environment
nix develop

# Run with auto-rebuild on file changes
cargo watch -x 'run -- --help'

# Run tests continuously
cargo watch -x test

# Run with Doppler secrets
doppler run -- cargo run -- fetch --source notion
```

### Code Quality Checks

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run linter
cargo clippy

# Security audit
cargo audit

# Run all checks at once
nix flake check
```

### Building for Production

```bash
# Build optimized release version
cargo build --release

# Or build with Nix for reproducible builds
nix build

# The binary will be in ./result/bin/mcp-rs
```

## Troubleshooting

### Common Issues

1. **"command not found: nix"**
   - Restart your shell or source the nix profile:
   ```bash
   . /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh
   ```

2. **SSL certificate errors**
   - The Nix environment should set these automatically, but you can manually set:
   ```bash
   export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
   ```

3. **API authentication errors**
   - Verify your API keys are correct:
   ```bash
   doppler secrets list
   ```
   - Make sure you've shared your Notion databases with the integration

4. **Build errors**
   - Make sure you're in the Nix development environment:
   ```bash
   nix develop
   ```

5. **Permission errors with Doppler**
   - Make sure you have access to the Doppler project:
   ```bash
   doppler projects list
   ```

### Getting Help

- Check the main [README.md](README.md) for detailed usage instructions
- Review the [CLAUDE.md](CLAUDE.md) file for development guidelines
- Look at the source code architecture in the `src/` directory

## Next Steps

1. **Explore the codebase**: The project uses hexagonal architecture - check out the `src/` directory structure
2. **Add new providers**: Follow the pattern in `src/infrastructure/adapters/` to add new API integrations
3. **Customize the CLI**: Modify the commands in `src/infrastructure/cli/` to add new functionality
4. **Set up CI/CD**: Use `nix flake check` in your CI pipeline for consistent builds

## Advanced Tips

### Using Direnv (Optional)

If you want the development environment to activate automatically when you enter the directory:

```bash
# Install direnv
nix profile install nixpkgs#direnv

# Create .envrc file
echo "use flake" > .envrc

# Allow direnv to activate
direnv allow
```

### Custom Environment Variables

You can add custom environment variables to the Nix shell by editing the `shellHook` in `flake.nix`.

### Using Different Rust Versions

The flake is configured to use the latest stable Rust. To use a different version, edit the `rustToolchain` definition in `flake.nix`.

---

🎉 **Congratulations!** You now have a fully functional MCP-RS development environment with secure secret management and all the tools you need for productive development.