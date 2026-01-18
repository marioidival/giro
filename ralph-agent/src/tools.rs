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
