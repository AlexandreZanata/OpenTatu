//! # opentatu-investigator — AI roles of the internal investigator
//!
//! Everything between "the worker decided to call role X" and "a validated JSON answer":
//!
//! - [`prompts`]: the versioned prompt files of `prompts/`, embedded in the binary.
//! - [`render`](mod@render): strict `{{variable}}` substitution (a missing value is an error).
//! - [`fence`](mod@fence): wraps untrusted content (code, scanner output, diffs) in tagged blocks
//!   that it cannot escape from (R11).
//! - [`outputs`]: output types shared by every role ([`CommonFields`]).
//!
//! Message layout of every role call: the **system** message is `base.md` + `<role>.md`
//! rendered with [`prompts::VARIABLES`]; the **user** message holds the role inputs, each
//! one fenced with [`fence::fence`].
//!
//! Phase 3/4 adds here: output types (serde + JSON Schema), validation and the retry loop
//! (at most 2 retries on invalid JSON, spec §C.7) on top of `opentatu_llm::Llm::generate`.

mod error;

pub mod fence;
pub mod outputs;
pub mod prompts;
pub mod render;

pub use error::Error;
pub use fence::{Block, fence};
pub use outputs::{CommonFields, SuspiciousInstruction};
pub use prompts::Prompt;
pub use render::render;
