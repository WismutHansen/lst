use anyhow::Result;
use lst_cli::cli::commands;
use lst_cli::storage;
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult};
use rust_mcp_sdk::{
    macros::{mcp_tool, JsonSchema},
    tool_box,
};

//****************//
//  ListListsTool  //
//****************//
#[mcp_tool(
    name = "list_lists",
    description = "lists the names of all available todo lists",
    idempotent_hint = false,
    destructive_hint = false,
    open_world_hint = false,
    read_only_hint = false
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct ListListsTool {}

impl ListListsTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        match storage::list_lists() {
            Ok(lists) => {
                if lists.is_empty() {
                    Ok(CallToolResult::text_content("No lists found.".to_string(), None))
                } else {
                    let lists_json = serde_json::to_string(&lists)
                        .map_err(|e| CallToolError::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to serialize lists: {}", e))))?;
                    Ok(CallToolResult::text_content(lists_json, None))
                }
            },
            Err(e) => Err(CallToolError::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e)))),
        }
    }
}

//******************//
//  AddToListTool  //
//******************//
#[mcp_tool(
    name = "add_to_list",
    description = "adds one or multiple items to a specified todo list, creates list if it does not yet exist",
    idempotent_hint = false,
    destructive_hint = false,
    open_world_hint = false,
    read_only_hint = false
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct AddToListTool {
    /// The name of the list to add the item to.
    list: String,
    /// The item to add to the list.
    item: String,
}
impl AddToListTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        match commands::add_item(&self.list, &self.item, false) {
            Ok(_) => Ok(CallToolResult::text_content(format!("Added '{}' to list '{}'", self.item, self.list), None)),
            Err(e) => Err(CallToolError::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e)))),
        }
    }
}

//******************//
//  GreetingTools  //
//******************//
// Generates an enum names GreetingTools, with SayHelloTool and SayGoodbyeTool variants
tool_box!(LstTools, [ListListsTool, AddToListTool]);
