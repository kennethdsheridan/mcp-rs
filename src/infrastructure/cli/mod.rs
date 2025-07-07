use clap::{Parser, Subcommand};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "mcp-rs")]
#[command(about = "A Model Context Protocol CLI for accessing multiple API resources")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Fetch resources from configured providers
    Fetch {
        /// Source provider (notion, linear, all)
        #[arg(short, long, default_value = "all")]
        source: String,
        
        /// Limit number of results
        #[arg(short, long)]
        limit: Option<usize>,
        
        /// Additional filters (key=value pairs)
        #[arg(short, long)]
        filter: Vec<String>,
    },
    
    /// Get a specific resource by ID
    Get {
        /// Resource ID
        id: String,
    },
    
    /// Search for resources
    Search {
        /// Search query
        query: String,
        
        /// Source providers to search (notion, linear, all)
        #[arg(short, long, default_value = "all")]
        source: Vec<String>,
        
        /// Limit number of results
        #[arg(short, long)]
        limit: Option<usize>,
    },
    
    /// List configured providers
    Providers,
    
    /// Configure API credentials
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set an API key
    Set {
        /// Provider name (notion, linear)
        provider: String,
        /// API key
        key: String,
    },
    
    /// List current configuration
    List,
    
    /// Test provider connections
    Test {
        /// Provider to test (optional, tests all if not specified)
        provider: Option<String>,
    },
}

pub fn parse_filters(filters: Vec<String>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for filter in filters {
        if let Some((key, value)) = filter.split_once('=') {
            map.insert(key.to_string(), value.to_string());
        }
    }
    map
}

pub fn parse_sources(sources: Vec<String>) -> Vec<crate::domain::QuerySource> {
    sources
        .into_iter()
        .map(|s| match s.to_lowercase().as_str() {
            "notion" => crate::domain::QuerySource::Notion,
            "linear" => crate::domain::QuerySource::Linear,
            "all" => crate::domain::QuerySource::All,
            _ => crate::domain::QuerySource::All,
        })
        .collect()
}