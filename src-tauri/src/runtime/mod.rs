//! Agent runtime: host state machine + agent loop.

pub mod agent_loop;
pub mod engine;
pub mod error;
pub mod events;
pub mod history;
pub mod mode;
pub mod tools;

pub use agent_loop::{
    execute_agent_run, execute_agent_run_after_grant_concurrent, execute_agent_run_concurrent,
    product_request_cancel, AgentLoopInput, AgentLoopOwned, MAX_TOOL_ROUNDS,
};
pub use engine::{AgentRuntime, RUNTIME_POLICY_VERSION};
pub use error::{
    ErrorCategory, ErrorOrigin, ErrorStage, Retryability, RuntimeError, SideEffectCertainty,
};
pub use mode::{mode_allows_tool_effect, Mode, ToolEffect};
pub use tools::{MockToolRunner, ToolOutcomeKind, ToolProposal, ToolResult};
