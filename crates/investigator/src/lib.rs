//! # opentatu-investigator — AI roles of the internal investigator
//!
//! Everything between "the worker decided to call role X" and "a validated JSON answer":
//!
//! - [`prompts`]: the versioned prompt files of `prompts/`, embedded in the binary.
//! - [`render`](mod@render): strict `{{variable}}` substitution (a missing value is an error).
//! - [`fence`](mod@fence): wraps untrusted content (code, scanner output, diffs) in tagged blocks
//!   that it cannot escape from (R11).
//! - [`outputs`]: output types shared by every role ([`CommonFields`]).
//! - [`run`](mod@run): one role call — render, fence, call, validate, retry at most
//!   [`run::MAX_RETRIES`] times ([`run_role`]).
//! - [`variables`]: base-prompt values of the running audit ([`RunContext`]).
//! - [`cartographer`]: the cartographer's inputs and output ([`CartographerOutput`]).
//!
//! Message layout of every role call: the **system** message is `base.md` + `<role>.md`
//! rendered with [`prompts::VARIABLES`]; the **user** message holds the role inputs, each
//! one fenced with [`fence::fence`].
//!
//! Role output types are serde types: deserialization is the schema check (ADR-0024).
//! Phase 4 adds the hunter, verifier and skeptic; phase 5 the planner and reporter.

mod error;

pub mod cartographer;
pub mod fence;
pub mod outputs;
pub mod prompts;
pub mod render;
pub mod run;
pub mod variables;

pub use cartographer::{CartographerInputs, CartographerOutput, asvs_listing};
pub use error::Error;
pub use fence::{Block, fence};
pub use outputs::{CommonFields, SuspiciousInstruction};
pub use prompts::Prompt;
pub use render::render;
pub use run::{RoleAnswer, RoleCall, RoleOutput, run_role};
pub use variables::RunContext;
