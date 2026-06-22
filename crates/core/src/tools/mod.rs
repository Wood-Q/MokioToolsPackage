//! Concrete [`Installer`](crate::installer::Installer) implementations, one per
//! tool. Each is a zero-sized (or near) value type; all state lives in the
//! environment.

pub mod cc_switch;
pub mod chrome;
pub mod claude_code;
pub mod codex;
pub mod git;
pub mod homebrew;
pub mod node;
pub mod terminal;
pub mod uv;
pub mod vscode;

pub use cc_switch::CcSwitch;
pub use chrome::Chrome;
pub use claude_code::ClaudeCode;
pub use codex::Codex;
pub use git::Git;
pub use homebrew::Homebrew;
pub use node::Node;
pub use terminal::Terminal;
pub use uv::Uv;
pub use vscode::VsCode;
