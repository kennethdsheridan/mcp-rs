{
  description = "MCP-RS: Model Context Protocol CLI tool built in Rust";

  inputs = {
    # Main nixpkgs for stable packages
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    
    # Rust overlay for latest rust toolchain
    rust-overlay.url = "github:oxalica/rust-overlay";
    
    # Flake utils for cross-platform support
    flake-utils.url = "github:numtide/flake-utils";
    
    # Crane for Rust builds in Nix
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # Import nixpkgs with rust overlay
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Define the rust toolchain we want to use
        # Using the latest stable rust with required components
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };

        # Create crane library with our rust toolchain
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Common arguments for all crane builds
        commonArgs = {
          # Source directory (filters out non-rust files for better caching)
          src = craneLib.cleanCargoSource ./.;
          
          # Build-time dependencies
          nativeBuildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            
            # Build tools
            pkg-config
            
            # For OpenSSL compilation
            openssl.dev
          ];

          # Runtime dependencies
          buildInputs = with pkgs; [
            # SSL/TLS support for HTTPS requests
            openssl
            
            # CA certificates for SSL verification
            cacert
            
            # System libraries that might be needed
            libiconv
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            # macOS-specific dependencies
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          # Environment variables for build
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
        };

        # Build the dependencies separately for better caching
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual application
        mcp-rs = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          
          # Package metadata
          pname = "mcp-rs";
          version = "0.1.0";
          
          # Additional build configuration
          doCheck = true;  # Run tests during build
          
          # Install phase customization
          postInstall = ''
            # Create a wrapper script that ensures proper environment
            mkdir -p $out/bin
            cat > $out/bin/mcp-rs-wrapper << 'EOF'
            #!/bin/bash
            # Set up SSL certificates
            export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
            export SSL_CERT_DIR=${pkgs.cacert}/etc/ssl/certs
            
            # Execute the real binary
            exec $out/bin/mcp-rs "$@"
            EOF
            chmod +x $out/bin/mcp-rs-wrapper
          '';
        });

        # Development shell with all tools and dependencies
        devShell = pkgs.mkShell {
          inputsFrom = [ mcp-rs ];
          
          # Development tools and dependencies
          buildInputs = with pkgs; [
            # Rust development tools
            rustToolchain
            
            # Additional development tools
            cargo-watch        # Auto-rebuild on file changes
            cargo-edit         # Cargo subcommands for editing Cargo.toml
            cargo-audit        # Security audit for dependencies
            cargo-outdated     # Check for outdated dependencies
            cargo-tarpaulin    # Code coverage tool
            
            # Secret management
            doppler            # Doppler CLI for secret management
            
            # Development utilities
            git                # Version control
            just               # Command runner (alternative to make)
            
            # API testing tools
            curl               # HTTP client
            jq                 # JSON processor
            
            # Documentation tools
            mdbook            # Markdown book generator
            
            # Environment and debugging
            direnv            # Environment variable management
            
            # SSL/TLS tools
            openssl           # SSL toolkit
            cacert            # CA certificates
            
            # Process management
            ps                # Process viewer
            
            # Network debugging
            netcat            # Network toolkit
            
            # System monitoring
            htop              # Process monitor
            
            # Text processing
            ripgrep           # Fast grep alternative
            fd                # Fast find alternative
            
            # Database tools (in case you need them for testing)
            sqlite            # Lightweight database
            
            # Container tools (useful for integration testing)
            docker            # Container runtime
            docker-compose    # Multi-container orchestration
          ];

          # Environment variables for development
          shellHook = ''
            # Welcome message
            echo "🦀 Welcome to MCP-RS development environment!"
            echo "📦 Rust toolchain: ${rustToolchain.name}"
            echo "🔧 Doppler CLI is available for secret management"
            echo ""
            echo "Available commands:"
            echo "  cargo build          - Build the project"
            echo "  cargo test           - Run tests"
            echo "  cargo run -- --help  - Run with help"
            echo "  doppler setup        - Set up Doppler for secrets"
            echo "  doppler run -- cargo run  - Run with Doppler secrets"
            echo ""
            
            # Set up SSL certificates
            export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
            export SSL_CERT_DIR=${pkgs.cacert}/etc/ssl/certs
            
            # Set up Rust environment
            export RUST_BACKTRACE=1
            export RUST_LOG=debug
            
            # Set up development paths
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig"
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
            export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
            
            # Create .env file template if it doesn't exist
            if [ ! -f .env ]; then
              cat > .env << 'EOF'
            # MCP-RS Environment Variables
            # Copy this file and fill in your actual values
            
            # Notion API Configuration
            NOTION_API_KEY=your_notion_integration_token_here
            
            # Linear API Configuration  
            LINEAR_API_KEY=your_linear_api_key_here
            
            # Logging Configuration
            RUST_LOG=info
            RUST_BACKTRACE=1
            EOF
              echo "📝 Created .env template file. Please fill in your API keys."
            fi
            
            # Check if doppler is configured
            if command -v doppler &> /dev/null; then
              echo "💡 Tip: Use 'doppler setup' to configure secret management"
              echo "💡 Then run: 'doppler run -- cargo run' to use managed secrets"
            fi
          '';
        };

      in
      {
        # The default package is our built application
        packages.default = mcp-rs;
        packages.mcp-rs = mcp-rs;
        
        # Development shell
        devShells.default = devShell;
        
        # Applications that can be run with 'nix run'
        apps.default = flake-utils.lib.mkApp {
          drv = mcp-rs;
          name = "mcp-rs";
        };
        
        # Checks for CI/CD
        checks = {
          # Build the package
          build = mcp-rs;
          
          # Run clippy (linting)
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --warn clippy::all";
          });
          
          # Run tests
          test = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
          });
          
          # Check formatting
          fmt = craneLib.cargoFmt {
            inherit (commonArgs) src;
          };
        };
        
        # Formatter for 'nix fmt'
        formatter = pkgs.nixpkgs-fmt;
      });
}