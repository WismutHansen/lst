#[cfg(test)]
mod tests {
    use super::super::tools::*;
    use lst_core::storage;
    use std::fs;
    use std::path::PathBuf;

    /// Helper to get the lists directory from config
    fn get_test_lists_dir() -> PathBuf {
        storage::get_lists_dir().expect("Failed to get lists directory")
    }

    #[test]
    fn test_list_lists_tool() {
        let tool = ListListsTool {};
        let result = tool.call_tool();
        
        // Should always succeed even if empty
        assert!(result.is_ok(), "ListListsTool failed: {:?}", result.err());
        
        let call_result = result.unwrap();
        assert!(!call_result.content.is_empty());
    }

    #[test]
    fn test_list_lists_with_items() {
        // Set up test lists in the proper directory from config
        let lists_dir = get_test_lists_dir();
        
        fs::write(lists_dir.join("test_mcp_groceries.md"), "- [ ] milk\n- [ ] bread\n").unwrap();
        fs::write(lists_dir.join("test_mcp_todo_list.md"), "- [ ] task1\n- [ ] task2\n").unwrap();

        let tool = ListListsTool {};
        let result = tool.call_tool();
        assert!(result.is_ok());

        let call_result = result.unwrap();
        assert!(!call_result.content.is_empty());
        
        // Clean up
        let _ = fs::remove_file(lists_dir.join("test_mcp_groceries.md"));
        let _ = fs::remove_file(lists_dir.join("test_mcp_todo_list.md"));
    }

    #[test]
    fn test_add_to_list_new_list() {
        let tool = AddToListTool {
            list: "test_mcp_shopping".to_string(),
            item: "apples".to_string(),
        };

        let result = tool.call_tool();
        assert!(result.is_ok(), "Failed to add item: {:?}", result.err());

        // Clean up
        let lists_dir = get_test_lists_dir();
        let _ = fs::remove_file(lists_dir.join("test_mcp_shopping.md"));
    }

    #[test]
    fn test_add_to_list_existing_list() {
        let lists_dir = get_test_lists_dir();
        
        fs::write(lists_dir.join("test_mcp_groceries2.md"), "- [ ] milk\n").unwrap();

        let tool = AddToListTool {
            list: "test_mcp_groceries2".to_string(),
            item: "bread".to_string(),
        };

        let result = tool.call_tool();
        assert!(result.is_ok(), "Failed to add item: {:?}", result.err());

        // Clean up
        let _ = fs::remove_file(lists_dir.join("test_mcp_groceries2.md"));
    }

    #[test]
    fn test_add_multiple_items() {
        let tool = AddToListTool {
            list: "test_mcp_shopping2".to_string(),
            item: "apples, oranges, bananas".to_string(),
        };

        let result = tool.call_tool();
        assert!(result.is_ok(), "Failed to add items: {:?}", result.err());

        // Clean up
        let lists_dir = get_test_lists_dir();
        let _ = fs::remove_file(lists_dir.join("test_mcp_shopping2.md"));
    }

    #[test]
    fn test_mark_done() {
        let lists_dir = get_test_lists_dir();
        
        fs::write(lists_dir.join("test_mcp_todo_mark.md"), "- [ ] task1\n- [ ] task2\n").unwrap();

        let tool = MarkDoneTool {
            list: "test_mcp_todo_mark".to_string(),
            target: "task1".to_string(),
        };

        let result = tool.call_tool();
        assert!(result.is_ok(), "Failed to mark done: {:?}", result.err());

        // Clean up
        let _ = fs::remove_file(lists_dir.join("test_mcp_todo_mark.md"));
    }

    #[test]
    fn test_mark_undone() {
        let lists_dir = get_test_lists_dir();

        let list_path = lists_dir.join("test_mcp_todo_unmark.md");
        fs::write(&list_path, "- [x] task1\n- [ ] task2\n").unwrap();

        let tool = MarkUndoneTool {
            list: "test_mcp_todo_unmark".to_string(),
            target: "task1".to_string(),
        };

        let result = tool.call_tool();
        assert!(result.is_ok(), "Failed to mark undone: {:?}", result.err());

        // Clean up
        let _ = fs::remove_file(&list_path);
    }

    #[test]
    fn test_mark_done_nonexistent_list() {
        let tool = MarkDoneTool {
            list: "test_mcp_nonexistent_list_12345".to_string(),
            target: "task1".to_string(),
        };

        let result = tool.call_tool();
        assert!(result.is_err(), "Should fail for nonexistent list");
    }

    #[test]
    fn test_mark_done_nonexistent_item() {
        let lists_dir = get_test_lists_dir();
        
        fs::write(lists_dir.join("test_mcp_todo_missing.md"), "- [ ] task1\n- [ ] task2\n").unwrap();

        let tool = MarkDoneTool {
            list: "test_mcp_todo_missing".to_string(),
            target: "nonexistent_item_xyz".to_string(),
        };

        let result = tool.call_tool();
        assert!(result.is_err(), "Should fail for nonexistent item");
        
        // Clean up
        let _ = fs::remove_file(lists_dir.join("test_mcp_todo_missing.md"));
    }

    #[test]
    fn test_add_to_list_with_special_characters() {
        let tool = AddToListTool {
            list: "test_mcp_notes".to_string(),
            item: "Buy @item with #tag".to_string(),
        };

        let result = tool.call_tool();
        assert!(result.is_ok(), "Failed to add item with special chars: {:?}", result.err());

        // Clean up
        let lists_dir = get_test_lists_dir();
        let _ = fs::remove_file(lists_dir.join("test_mcp_notes.md"));
    }

    #[test]
    fn test_list_lists_tool_serialization() {
        let tool = ListListsTool {};
        let json = serde_json::to_string(&tool).expect("Failed to serialize");
        let _deserialized: ListListsTool =
            serde_json::from_str(&json).expect("Failed to deserialize");
        // Just verify it round-trips successfully
    }

    #[test]
    fn test_add_to_list_tool_serialization() {
        let tool = AddToListTool {
            list: "test".to_string(),
            item: "item1".to_string(),
        };
        let json = serde_json::to_string(&tool).expect("Failed to serialize");
        let _deserialized: AddToListTool =
            serde_json::from_str(&json).expect("Failed to deserialize");
        // Verify round-trip serialization works
    }
}
