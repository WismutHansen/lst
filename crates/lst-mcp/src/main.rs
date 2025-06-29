use std::error::Error;

use lst_cli::cli::commands::{
    add_item as cli_add_item_to_list, mark_done as cli_mark_item_done,
    mark_undone as cli_mark_item_undone, new_list as cli_create_new_list,
};

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, serve_server,
};

#[derive(Debug, Clone)]
pub struct LstMcp {
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, schemars::JsonSchema, serde::Deserialize, serde::Serialize)]
pub struct CreateListRequest {
    pub name: String,
}

#[derive(Debug, schemars::JsonSchema, serde::Deserialize, serde::Serialize)]
pub struct AddItemRequest {
    pub list_name: String,
    pub item_text: String,
}

#[derive(Debug, schemars::JsonSchema, serde::Deserialize, serde::Serialize)]
pub struct MarkItemRequest {
    pub list_name: String,
    pub item_text: String,
}

#[tool_router]
impl LstMcp {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Create a new list")]
    pub async fn create_new_list(
        &self,
        Parameters(CreateListRequest { name }): Parameters<CreateListRequest>,
    ) -> String {
        match cli_create_new_list(&name) {
            Ok(_) => format!("List '{}' created successfully.", name),
            Err(e) => format!("Error creating list '{}': {}", name, e),
        }
    }

    #[tool(description = "Add an item to a list")]
    pub async fn add_item_to_list(
        &self,
        Parameters(AddItemRequest { list_name, item_text }): Parameters<AddItemRequest>,
    ) -> String {
        match cli_add_item_to_list(&list_name, &item_text, false) {
            Ok(_) => format!("Item '{}' added to list '{}'.", item_text, list_name),
            Err(e) => format!("Error adding item to list '{}': {}", list_name, e),
        }
    }

    #[tool(description = "Mark an item as done")]
    pub async fn mark_item_done(
        &self,
        Parameters(MarkItemRequest { list_name, item_text }): Parameters<MarkItemRequest>,
    ) -> String {
        match cli_mark_item_done(&list_name, &item_text, false) {
            Ok(_) => format!("Item '{}' marked as done in list '{}'.", item_text, list_name),
            Err(e) => format!("Error marking item done in list '{}': {}", list_name, e),
        }
    }

    #[tool(description = "Mark an item as undone")]
    pub async fn mark_item_undone(
        &self,
        Parameters(MarkItemRequest { list_name, item_text }): Parameters<MarkItemRequest>,
    ) -> String {
        match cli_mark_item_undone(&list_name, &item_text, false) {
            Ok(_) => format!("Item '{}' marked as undone in list '{}'.", item_text, list_name),
            Err(e) => format!("Error marking item undone in list '{}': {}", list_name, e),
        }
    }
}

#[tool_handler]
impl ServerHandler for LstMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple list manager for creating lists and managing items".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let lst_service = LstMcp::new();

    println!("Starting LST MCP server, connect to standard input/output");

    let io = (tokio::io::stdin(), tokio::io::stdout());

    serve_server(lst_service, io).await?;
    Ok(())
}