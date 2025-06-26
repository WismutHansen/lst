use lst_cli::cli::commands::list_lists;
use lst_cli::models::list::{List, ListMetadata};
use rmcp::{
    Error as McpError, ServiceExt, handler::server::router::tool::ToolRouter,
    handler::server::tool::ToolCallContext, model::*, tool, tool_router, transport::stdio,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct LstList {
    name: str,
    items: Vec<ListItem>
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl LstList {
    fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool( description = ("get a list of all available lists"))]
    async fn lst_list_lists() -> Result<()> {
        let lists = list_lists(false);
    }

}

// Implement the server handler
#[tool_handler]
impl rmcp::ServerHandler for Counter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// Run the server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and run the server with STDIO transport
    let service = Counter::new().serve(stdio()).await.inspect_err(|e| {
        println!("Error starting server: {}", e);
    })?;
    service.waiting().await?;

    Ok(())
}
