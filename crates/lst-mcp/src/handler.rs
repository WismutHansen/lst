use async_trait::async_trait;
use rust_mcp_sdk::schema::{
    schema_utils::CallToolError, CallToolRequest, CallToolResult, ListToolsRequest,
    ListToolsResult, RpcError,
};
use rust_mcp_sdk::{mcp_server::ServerHandler, McpServer};
use std::sync::Arc;

use crate::tools::LstTools;

// Custom Handler to handle MCP Messages
pub struct MyServerHandler;

// To check out a list of all the methods in the trait that you can override, take a look at
// https://github.com/rust-mcp-stack/rust-mcp-sdk/blob/main/crates/rust-mcp-sdk/src/mcp_handlers/mcp_server_handler.rs

#[async_trait]
#[allow(unused)]
impl ServerHandler for MyServerHandler {
    // Handle ListToolsRequest, return list of available tools as ListToolsResult
    async fn handle_list_tools_request(
        &self,
        request: ListToolsRequest,
        runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        tracing::debug!("Handling list_tools request");
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: LstTools::tools(),
        })
    }

    /// Handles incoming CallToolRequest and processes it using the appropriate tool.
    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        tracing::debug!("Handling call_tool request: {:?}", request.params.name);
        // Attempt to convert request parameters into LstTools enum
        let tool_params: LstTools =
            LstTools::try_from(request.params).map_err(|e| {
                tracing::error!("Failed to parse tool parameters: {:?}", e);
                CallToolError::new(e)
            })?;

        // Match the tool variant and execute its corresponding logic
        match tool_params {
            LstTools::ListListsTool(list_list_tool) => list_list_tool.call_tool(),
            LstTools::AddToListTool(add_to_list_tool) => add_to_list_tool.call_tool(),
            LstTools::MarkDoneTool(mark_done_tool) => mark_done_tool.call_tool(),
            LstTools::MarkUndoneTool(mark_undone_tool) => mark_undone_tool.call_tool(),
        }
    }
}
