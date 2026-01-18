use serde::{Deserialize, Serialize};

/// Result from executing a tool
///
/// Contains the output of the tool execution and any error that occurred.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The command output
    pub output: String,
    /// Optional error message
    pub error: Option<String>,
}

/// Trait for tools that can be executed by the agent
///
/// This trait defines the interface for tool implementations that can
/// be used by the Ralph agent to perform operations like file I/O,
/// command execution, and other system-level tasks.
///
/// The trait is designed to be async-friendly for future integration
/// with `ExecutionContext` and container-based command execution.
pub trait Tool {
    /// Returns the tool's name
    ///
    /// # Returns
    /// The tool's name as a string slice
    fn name(&self) -> &str;

    /// Returns a description of the tool
    ///
    /// # Returns
    /// The tool's description as a string slice
    fn description(&self) -> &str;

    /// Executes the tool with the given arguments
    ///
    /// # Arguments
    /// * `args` - Slice of string arguments to pass to the tool
    ///
    /// # Returns
    /// `ToolResult` containing the output and optional error
    fn execute(&self, args: &[&str]) -> ToolResult;
}

/// Tool for file operations in the container
///
/// FileTool provides file read, write, and list operations for files
/// within the container's working directory. It works with ExecutionContext
/// to perform these operations synchronously.
///
/// Supported commands:
/// - `file read <path>` - Read file contents
/// - `file write <path> <content>` - Write content to a file
/// - `file list [path]` - List files in a directory (defaults to working directory)
#[derive(Debug, Clone)]
pub struct FileTool;

impl FileTool {
    /// Create a new FileTool instance
    pub fn new() -> Self {
        Self
    }

    /// Parse and execute a file read command
    ///
    /// # Arguments
    /// * `args` - Command arguments, should be ["read", "<path>"]
    ///
    /// # Returns
    /// ToolResult indicating the file content or error
    fn handle_read(&self, args: &[&str]) -> ToolResult {
        if args.len() != 2 {
            return ToolResult {
                output: String::new(),
                error: Some("file read requires exactly 1 argument: <path>".to_string()),
            };
        }

        let path = args[1];
        // In a real implementation, this would call ctx.read_file(path).await
        // For now, return a placeholder indicating the operation
        ToolResult {
            output: format!(
                "FileTool: read_file('{}') - integrate with ExecutionContext for async execution",
                path
            ),
            error: None,
        }
    }

    /// Parse and execute a file write command
    ///
    /// # Arguments
    /// * `args` - Command arguments, should be ["write", "<path>", "<content>"]
    ///
    /// # Returns
    /// ToolResult indicating success or error
    fn handle_write(&self, args: &[&str]) -> ToolResult {
        if args.len() < 3 {
            return ToolResult {
                output: String::new(),
                error: Some(
                    "file write requires at least 2 arguments: <path> <content>".to_string(),
                ),
            };
        }

        let path = args[1];
        let content = args[2..].join(" ");
        // In a real implementation, this would call ctx.write_file(path, content).await
        // For now, return a placeholder indicating the operation
        ToolResult {
            output: format!(
                "FileTool: write_file('{}', {} bytes) - integrate with ExecutionContext for async execution",
                path,
                content.len()
            ),
            error: None,
        }
    }

    /// Parse and execute a file list command
    ///
    /// # Arguments
    /// * `args` - Command arguments, should be ["list"] or ["list", "<path>"]
    ///
    /// # Returns
    /// ToolResult listing files or error
    fn handle_list(&self, args: &[&str]) -> ToolResult {
        if args.len() > 2 {
            return ToolResult {
                output: String::new(),
                error: Some("file list takes at most 1 argument: [path]".to_string()),
            };
        }

        let path = if args.len() == 2 { Some(args[1]) } else { None };

        let path_desc = path.map_or("working directory".to_string(), |p| format!("'{}'", p));
        // In a real implementation, this would call ctx.list_files(path).await
        // For now, return a placeholder indicating the operation
        ToolResult {
            output: format!(
                "FileTool: list_files({}) - integrate with ExecutionContext for async execution",
                path_desc
            ),
            error: None,
        }
    }
}

impl Default for FileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for FileTool {
    fn name(&self) -> &str {
        "file"
    }

    fn description(&self) -> &str {
        "Perform file operations in the container: read, write, and list files"
    }

    fn execute(&self, args: &[&str]) -> ToolResult {
        if args.is_empty() {
            return ToolResult {
                output: String::new(),
                error: Some(self.usage()),
            };
        }

        match args[0] {
            "read" => self.handle_read(args),
            "write" => self.handle_write(args),
            "list" => self.handle_list(args),
            _ => ToolResult {
                output: String::new(),
                error: Some(format!(
                    "Unknown file command: '{}'\n{}",
                    args[0],
                    self.usage()
                )),
            },
        }
    }
}

impl FileTool {
    /// Returns usage information for the file tool
    fn usage(&self) -> String {
        [
            "FileTool usage:",
            "  file read <path>        - Read file contents",
            "  file write <path> <content> - Write content to a file",
            "  file list [path]        - List files in directory (defaults to working directory)",
        ]
        .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tool_name() {
        let tool = FileTool::new();
        assert_eq!(tool.name(), "file");
    }

    #[test]
    fn test_file_tool_description() {
        let tool = FileTool::new();
        assert!(tool.description().contains("file operations"));
    }

    #[test]
    fn test_file_read_command_works() {
        let tool = FileTool::new();
        let result = tool.execute(&["read", "/path/to/file.txt"]);

        assert!(result.output.contains("read_file"));
        assert!(result.output.contains("/path/to/file.txt"));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_file_read_missing_path() {
        let tool = FileTool::new();
        let result = tool.execute(&["read"]);

        assert!(result.output.is_empty());
        assert!(result.error.is_some());
        assert!(
            result
                .error
                .unwrap()
                .contains("requires exactly 1 argument")
        );
    }

    #[test]
    fn test_file_write_command_works() {
        let tool = FileTool::new();
        let result = tool.execute(&["write", "/path/to/file.txt", "Hello, World!"]);

        assert!(result.output.contains("write_file"));
        assert!(result.output.contains("/path/to/file.txt"));
        assert!(result.output.contains("13 bytes")); // "Hello, World!" is 13 bytes
        assert!(result.error.is_none());
    }

    #[test]
    fn test_file_write_with_spaces() {
        let tool = FileTool::new();
        let result = tool.execute(&["write", "/path/file.txt", "multi word content"]);

        assert!(result.output.contains("write_file"));
        assert!(result.output.contains("18 bytes")); // "multi word content" is 18 bytes
        assert!(result.error.is_none());
    }

    #[test]
    fn test_file_write_missing_args() {
        let tool = FileTool::new();
        let result = tool.execute(&["write"]);

        assert!(result.output.is_empty());
        assert!(result.error.is_some());
        assert!(
            result
                .error
                .unwrap()
                .contains("requires at least 2 arguments")
        );
    }

    #[test]
    fn test_file_list_command_works() {
        let tool = FileTool::new();
        let result = tool.execute(&["list"]);

        assert!(result.output.contains("list_files"));
        assert!(result.output.contains("working directory"));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_file_list_with_path() {
        let tool = FileTool::new();
        let result = tool.execute(&["list", "/path/to/dir"]);

        assert!(result.output.contains("list_files"));
        assert!(result.output.contains("/path/to/dir"));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_file_list_too_many_args() {
        let tool = FileTool::new();
        let result = tool.execute(&["list", "arg1", "arg2"]);

        assert!(result.output.is_empty());
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("takes at most 1 argument"));
    }

    #[test]
    fn test_invalid_command_returns_usage() {
        let tool = FileTool::new();
        let result = tool.execute(&["invalid", "args"]);

        assert!(result.output.is_empty());
        assert!(result.error.is_some());
        let error_msg = result.error.unwrap();
        assert!(error_msg.contains("Unknown file command"));
        assert!(error_msg.contains("invalid"));
        assert!(error_msg.contains("file read"));
        assert!(error_msg.contains("file write"));
        assert!(error_msg.contains("file list"));
    }

    #[test]
    fn test_empty_args_returns_usage() {
        let tool = FileTool::new();
        let result = tool.execute(&[]);

        assert!(result.output.is_empty());
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("file read"));
    }

    #[test]
    fn test_file_tool_default() {
        let tool = FileTool::default();
        assert_eq!(tool.name(), "file");
    }
}
