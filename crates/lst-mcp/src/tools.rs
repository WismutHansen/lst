use anyhow::Result;
use lst_core::{commands, storage};
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
    description = "lists the names of all available lists (could be todo lists, shopping lists etc)",
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
                    Ok(CallToolResult::text_content(
                        "No lists found.".to_string(),
                        None,
                    ))
                } else {
                    let lists_json = serde_json::to_string(&lists).map_err(|e| {
                        CallToolError::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to serialize lists: {}", e),
                        ))
                    })?;
                    Ok(CallToolResult::text_content(lists_json, None))
                }
            }
            Err(e) => Err(CallToolError::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{}", e),
            ))),
        }
    }
}

//******************//
//  AddToListTool  //
//******************//
#[mcp_tool(
    name = "add_to_list",
    description = "adds one or multiple items to a specified list, creates list if it does not yet exist",
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
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            CallToolError::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create runtime: {}", e),
            ))
        })?;

        match rt.block_on(commands::add_item(&self.list, &self.item, false)) {
            Ok(_) => Ok(CallToolResult::text_content(
                format!("Added '{}' to list '{}'", self.item, self.list),
                None,
            )),
            Err(e) => Err(CallToolError::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{}", e),
            ))),
        }
    }
}

//******************//
//  MarkDoneTool   //
//******************//
#[mcp_tool(
    name = "mark_done",
    description = "marks one or more items as done in a specified list",
    idempotent_hint = false,
    destructive_hint = false,
    open_world_hint = false,
    read_only_hint = false
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct MarkDoneTool {
    /// The name of the list containing the item(s) to mark as done.
    list: String,
    /// The target item(s) to mark as done (can be anchor, text, index, or comma-separated multiple items).
    target: String,
}

impl MarkDoneTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            CallToolError::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create runtime: {}", e),
            ))
        })?;

        match rt.block_on(commands::mark_done(&self.list, &self.target, false)) {
            Ok(_) => Ok(CallToolResult::text_content(
                format!("Marked '{}' as done in list '{}'", self.target, self.list),
                None,
            )),
            Err(e) => Err(CallToolError::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{}", e),
            ))),
        }
    }
}

//******************//
//  MarkUndoneTool //
//******************//
#[mcp_tool(
    name = "mark_undone",
    description = "marks one or more completed items as not done in a specified list",
    idempotent_hint = false,
    destructive_hint = false,
    open_world_hint = false,
    read_only_hint = false
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct MarkUndoneTool {
    /// The name of the list containing the item(s) to mark as undone.
    list: String,
    /// The target item(s) to mark as undone (can be anchor, text, index, or comma-separated multiple items).
    target: String,
}

impl MarkUndoneTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            CallToolError::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create runtime: {}", e),
            ))
        })?;

        match rt.block_on(commands::mark_undone(&self.list, &self.target, false)) {
            Ok(_) => Ok(CallToolResult::text_content(
                format!("Marked '{}' as undone in list '{}'", self.target, self.list),
                None,
            )),
            Err(e) => Err(CallToolError::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{}", e),
            ))),
        }
    }
}

//******************//
//  LstTools Enum  //
//******************//
// Generates an enum names LstTools, with all tool variants
tool_box!(
    LstTools,
    [ListListsTool, AddToListTool, MarkDoneTool, MarkUndoneTool]
);
