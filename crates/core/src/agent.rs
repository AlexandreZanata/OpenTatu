//! Fixing agents and their MCP tokens (spec §E.6; ADR-0012; R16).
//!
//! Each agent has its own token (`otk_<64 hex>`). Only its hash (`sha256:<hex>`) is
//! stored; the token is shown once to the owner when the agent is registered and reaches
//! `opentatu mcp` through the agent's MCP configuration environment. The token is a
//! [`Secret`]: it never appears in logs, errors or events.

use serde::{Deserialize, Serialize};

use crate::fingerprint::{push_hex, sha256_text};
use crate::{Actor, Error, Secret, Timestamp};

/// Random bytes in a token.
pub const TOKEN_BYTES: usize = 32;

const TOKEN_PREFIX: &str = "otk_";

/// The credential of one fixing agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentToken(Secret);

impl AgentToken {
    /// Builds a token from random bytes (the caller supplies the randomness).
    pub fn from_random(bytes: [u8; TOKEN_BYTES]) -> Self {
        let mut text = String::with_capacity(TOKEN_PREFIX.len() + 2 * TOKEN_BYTES);
        text.push_str(TOKEN_PREFIX);
        push_hex(&mut text, &bytes);
        Self(Secret::new(text))
    }

    /// Parses a token received from an agent (`otk_<64 lowercase hex>`).
    pub fn parse(text: &str) -> Result<Self, Error> {
        let valid = text.strip_prefix(TOKEN_PREFIX).is_some_and(|hex| {
            hex.len() == 2 * TOKEN_BYTES
                && hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        });
        if valid {
            Ok(Self(Secret::new(text)))
        } else {
            // Never echo the value: it may be a real credential with a typo.
            Err(Error::InvalidId {
                kind: "AgentToken",
                value: crate::secrets::MASK.to_owned(),
                format: "otk_<64 lowercase hex chars>",
            })
        }
    }

    /// The hash stored in the database (`sha256:<64 hex>`).
    pub fn hash(&self) -> String {
        sha256_text(self.0.expose().as_bytes())
    }

    /// The token text, to show once to the owner.
    pub fn expose(&self) -> &str {
        self.0.expose()
    }
}

/// A registered fixing agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// Name (letters, digits, `-`, `_`, `.`); the actor is `agent:<name>`.
    pub name: String,
    /// When it was registered (clock).
    pub registered_at: Timestamp,
    /// When its token was revoked (clock), if it was.
    pub revoked_at: Option<Timestamp>,
}

impl Agent {
    /// Validates an agent name (the rules of the `agent:<id>` actor).
    pub fn validate_name(name: &str) -> Result<(), Error> {
        Actor::external(name).map(|_| ())
    }

    /// The actor of this agent's events (`agent:<name>`).
    pub fn actor(&self) -> Actor {
        Actor::ExternalAgent(format!("agent:{}", self.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r16_tokens_round_trip_and_never_print() {
        let token = AgentToken::from_random([0xab; TOKEN_BYTES]);
        assert!(token.expose().starts_with("otk_abab"));
        assert_eq!(AgentToken::parse(token.expose()).unwrap(), token);
        assert!(!format!("{token:?}").contains("abab"));
        assert!(token.hash().starts_with("sha256:"));
        assert_ne!(
            token.hash(),
            AgentToken::from_random([0; TOKEN_BYTES]).hash()
        );
        let error = AgentToken::parse("otk_123").unwrap_err().to_string();
        assert!(!error.contains("123"), "{error}");
        assert!(AgentToken::parse(&token.expose().to_uppercase()).is_err());
    }

    #[test]
    fn agent_names_follow_the_actor_rules() {
        assert!(Agent::validate_name("claude-code").is_ok());
        assert!(Agent::validate_name("codex_1.2").is_ok());
        assert!(Agent::validate_name("").is_err());
        assert!(Agent::validate_name("a b").is_err());
        let agent = Agent {
            name: "codex".into(),
            registered_at: "2026-11-12T11:00:00-03:00".parse().unwrap(),
            revoked_at: None,
        };
        assert_eq!(agent.actor().as_str(), "agent:codex");
    }
}
