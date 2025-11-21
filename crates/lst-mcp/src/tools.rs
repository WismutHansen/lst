use anyhow::Result;
use lst_core::{commands, storage};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
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
        tracing::debug!("ListListsTool: Listing all lists");
        match storage::list_lists() {
            Ok(lists) => {
                if lists.is_empty() {
                    Ok(CallToolResult::text_content(vec![TextContent::new(
                        "No lists found.".to_string(),
                        None,
                        None,
                    )]))
                } else {
                    let lists_json = serde_json::to_string(&lists).map_err(|e| {
                        CallToolError::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to serialize lists: {}", e),
                        ))
                    })?;
                    Ok(CallToolResult::text_content(vec![TextContent::new(
                        lists_json,
                        None,
                        None,
                    )]))
                }
            }
            Err(e) => {
                tracing::error!("ListListsTool: Failed to list lists: {}", e);
                Err(CallToolError::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to list lists: {}", e),
                )))
            }
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
    pub list: String,
    /// The item to add to the list.
    pub item: String,
}
impl AddToListTool {
    pub async fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        tracing::debug!("AddToListTool: Adding '{}' to list '{}'", self.item, self.list);

        match commands::add_item(&self.list, &self.item, false).await {
            Ok(_) => {
                tracing::info!("AddToListTool: Successfully added '{}' to list '{}'", self.item, self.list);
                Ok(CallToolResult::text_content(vec![TextContent::new(
                    format!("Added '{}' to list '{}'", self.item, self.list),
                    None,
                    None,
                )]))
            }
            Err(e) => {
                tracing::error!("AddToListTool: Failed to add item: {}", e);
                Err(CallToolError::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to add item: {}", e),
                )))
            }
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
    pub list: String,
    /// The target item(s) to mark as done (can be anchor, text, index, or comma-separated multiple items).
    pub target: String,
}

impl MarkDoneTool {
    pub async fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        tracing::debug!("MarkDoneTool: Marking '{}' as done in list '{}'", self.target, self.list);

        match commands::mark_done(&self.list, &self.target, false).await {
            Ok(_) => {
                tracing::info!("MarkDoneTool: Successfully marked '{}' as done in list '{}'", self.target, self.list);
                Ok(CallToolResult::text_content(vec![TextContent::new(
                    format!("Marked '{}' as done in list '{}'", self.target, self.list),
                    None,
                    None,
                )]))
            }
            Err(e) => {
                tracing::error!("MarkDoneTool: Failed to mark done: {}", e);
                Err(CallToolError::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to mark done: {}", e),
                )))
            }
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
    pub list: String,
    /// The target item(s) to mark as undone (can be anchor, text, index, or comma-separated multiple items).
    pub target: String,
}

impl MarkUndoneTool {
    pub async fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        tracing::debug!("MarkUndoneTool: Marking '{}' as undone in list '{}'", self.target, self.list);

        match commands::mark_undone(&self.list, &self.target, false).await {
            Ok(_) => {
                tracing::info!("MarkUndoneTool: Successfully marked '{}' as undone in list '{}'", self.target, self.list);
                Ok(CallToolResult::text_content(vec![TextContent::new(
                    format!("Marked '{}' as undone in list '{}'", self.target, self.list),
                    None,
                    None,
                )]))
            }
            Err(e) => {
                tracing::error!("MarkUndoneTool: Failed to mark undone: {}", e);
                Err(CallToolError::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to mark undone: {}", e),
                )))
            }
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
