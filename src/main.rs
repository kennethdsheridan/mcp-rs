mod domain;
mod ports;
mod application;
mod infrastructure;

use anyhow::Result;
use clap::Parser;
use dotenv::dotenv;
use std::{env, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    application::ResourceService,
    domain::{Query, QuerySource},
    infrastructure::{
        adapters::{notion::NotionAdapter, linear::LinearAdapter},
        cli::{Cli, Commands, ConfigAction, parse_filters, parse_sources},
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    
    let cli = Cli::parse();
    
    // Initialize tracing
    let filter = if cli.verbose {
        "mcp_rs=debug,info"
    } else {
        "mcp_rs=info,warn,error"
    };
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(filter))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize resource service
    let mut service = ResourceService::new();
    
    // Configure providers based on environment variables
    if let Ok(notion_key) = env::var("NOTION_API_KEY") {
        match NotionAdapter::new(notion_key) {
            Ok(adapter) => {
                service.add_provider(Arc::new(adapter));
                tracing::info!("Notion provider configured");
            }
            Err(e) => tracing::warn!("Failed to configure Notion provider: {}", e),
        }
    }
    
    if let Ok(linear_key) = env::var("LINEAR_API_KEY") {
        match LinearAdapter::new(linear_key) {
            Ok(adapter) => {
                service.add_provider(Arc::new(adapter));
                tracing::info!("Linear provider configured");
            }
            Err(e) => tracing::warn!("Failed to configure Linear provider: {}", e),
        }
    }

    // Handle commands
    match cli.command {
        Commands::Fetch { source, limit, filter } => {
            let query_source = match source.to_lowercase().as_str() {
                "notion" => QuerySource::Notion,
                "linear" => QuerySource::Linear,
                _ => QuerySource::All,
            };
            
            let filters = parse_filters(filter);
            let query = Query {
                source: query_source,
                filters,
                limit,
            };
            
            match service.fetch_resources(&query).await {
                Ok(resources) => {
                    println!("Found {} resources:", resources.len());
                    for resource in resources {
                        println!("\n--- {} ---", resource.title);
                        println!("ID: {}", resource.id);
                        println!("Source: {:?}", resource.source);
                        println!("Created: {}", resource.created_at);
                        println!("Content: {}", 
                            if resource.content.len() > 200 {
                                format!("{}...", &resource.content[..200])
                            } else {
                                resource.content
                            }
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching resources: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Get { id } => {
            match service.fetch_resource_by_id(&id).await {
                Ok(resource) => {
                    println!("Resource: {}", resource.title);
                    println!("ID: {}", resource.id);
                    println!("Source: {:?}", resource.source);
                    println!("Created: {}", resource.created_at);
                    println!("Updated: {}", resource.updated_at);
                    println!("\nContent:\n{}", resource.content);
                    
                    if !resource.metadata.is_empty() {
                        println!("\nMetadata:");
                        for (key, value) in resource.metadata {
                            println!("  {}: {}", key, value);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching resource: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Search { query, source, limit } => {
            let query_sources = parse_sources(source);
            
            match service.search(&query, Some(query_sources)).await {
                Ok(resources) => {
                    let display_limit = limit.unwrap_or(resources.len());
                    println!("Found {} resources (showing first {}):", resources.len(), display_limit.min(resources.len()));
                    
                    for resource in resources.into_iter().take(display_limit) {
                        println!("\n--- {} ---", resource.title);
                        println!("ID: {}", resource.id);
                        println!("Source: {:?}", resource.source);
                        println!("Content: {}", 
                            if resource.content.len() > 150 {
                                let truncated = resource.content.char_indices()
                                    .nth(150)
                                    .map(|(i, _)| &resource.content[..i])
                                    .unwrap_or(&resource.content);
                                format!("{}...", truncated)
                            } else {
                                resource.content
                            }
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Error searching resources: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Providers => {
            let providers = service.list_providers();
            if providers.is_empty() {
                println!("No providers configured. Set API keys in environment variables:");
                println!("  NOTION_API_KEY - for Notion integration");
                println!("  LINEAR_API_KEY - for Linear integration");
            } else {
                println!("Configured providers:");
                for provider in providers {
                    println!("  - {}", provider);
                }
            }
        }
        
        Commands::Config { action } => {
            match action {
                ConfigAction::Set { provider, key } => {
                    println!("To set API keys, use environment variables:");
                    match provider.to_lowercase().as_str() {
                        "notion" => println!("export NOTION_API_KEY=\"{}\"", key),
                        "linear" => println!("export LINEAR_API_KEY=\"{}\"", key),
                        _ => println!("Unknown provider: {}", provider),
                    }
                }
                
                ConfigAction::List => {
                    println!("Configuration:");
                    println!("  NOTION_API_KEY: {}", 
                        if env::var("NOTION_API_KEY").is_ok() { "✓ Set" } else { "✗ Not set" }
                    );
                    println!("  LINEAR_API_KEY: {}", 
                        if env::var("LINEAR_API_KEY").is_ok() { "✓ Set" } else { "✗ Not set" }
                    );
                }
                
                ConfigAction::Test { provider } => {
                    println!("Testing provider connections...");
                    let providers_to_test = if let Some(p) = provider {
                        vec![p]
                    } else {
                        vec!["notion".to_string(), "linear".to_string()]
                    };
                    
                    for provider_name in providers_to_test {
                        // Simple test by trying to search for a basic query
                        let query_source = match provider_name.as_str() {
                            "notion" => QuerySource::Notion,
                            "linear" => QuerySource::Linear,
                            _ => continue,
                        };
                        
                        match service.search("test", Some(vec![query_source])).await {
                            Ok(_) => println!("  {}: ✓ Connected", provider_name),
                            Err(e) => println!("  {}: ✗ Failed ({})", provider_name, e),
                        }
                    }
                }
            }
        }
    }

    Ok(())
}