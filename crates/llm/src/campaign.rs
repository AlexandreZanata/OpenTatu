//! Durable opt-in campaign accounting (R13, R17). Reserve before every network attempt;
//! unknown outcomes retain their entire reservation, including after a crash. The ledger
//! contains billing metadata only, never prompts, code or credentials (R15, R16).

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Mutex;

use opentatu_core::{AiRole, AuditId};
use serde::{Deserialize, Serialize};

use crate::{CallContext, CampaignPricing, Error, ModelRequest, ModelResponse, ModelsConfig};

const MICRO_USD: u64 = 1_000_000;

/// One exclusively locked, append-only campaign ledger, shared by all cloned routers.
/// Reuse the same file for every run belonging to the owner's aggregate spending cap.
#[derive(Debug)]
pub struct CampaignBudget {
    pricing: CampaignPricing,
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    file: File,
    cap: u64,
    committed: u64,
    next_attempt: u64,
    pending: BTreeMap<u64, u64>,
    halted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
enum Entry {
    Policy {
        version: u32,
        cap_micro_usd: u64,
        endpoint: String,
        pricing: CampaignPricing,
    },
    Reserved {
        attempt: u64,
        reserved_micro_usd: u64,
        audit_id: Option<AuditId>,
        role: Option<AiRole>,
    },
    Settled {
        attempt: u64,
        charged_micro_usd: u64,
        input_tokens: u64,
        output_tokens: u64,
    },
    Halted,
}

fn refused(reason: impl Into<String>) -> Error {
    Error::CampaignBudget(reason.into())
}

impl CampaignBudget {
    /// Opens or creates a 0600 JSONL ledger and holds an exclusive OS file lock until
    /// dropped. Existing cap, endpoint and pricing must match exactly; unknown/torn records
    /// fail closed. No owner data or secret values are written to this file.
    pub fn open(path: &Path, max_cost_usd: f64, config: &ModelsConfig) -> Result<Self, Error> {
        if !config.validate().is_empty() {
            return Err(refused(
                "campaign requires a valid researched models config",
            ));
        }
        let cap = dollars(max_cost_usd)?.min((max_cost_usd * MICRO_USD as f64).floor() as u64);
        if cap == 0 {
            return Err(refused("maximum cost must be positive"));
        }
        let pricing = config
            .campaign
            .clone()
            .ok_or_else(|| refused("models config has no researched campaign pricing"))?;
        let per_attempt = ceiling(
            &pricing,
            pricing.max_input_tokens,
            pricing.max_output_tokens.into(),
        )?;
        if per_attempt == 0 {
            return Err(refused("campaign pricing ceiling must be positive"));
        }
        let parent = path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let parent = parent
            .canonicalize()
            .map_err(|_| refused("campaign parent directory must already exist"))?;
        let path = parent.join(
            path.file_name()
                .ok_or_else(|| refused("campaign ledger must name a file"))?,
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            if std::fs::metadata(&parent)
                .map_err(|_| refused("cannot inspect campaign directory"))?
                .mode()
                & 0o077
                != 0
            {
                return Err(refused(
                    "campaign parent directory must be owner-only (0700)",
                ));
            }
        }
        if let Ok(metadata) = std::fs::symlink_metadata(&path)
            && !metadata.is_file()
        {
            return Err(refused("ledger must be a regular file, never a symlink"));
        }
        let mut options = OpenOptions::new();
        options.read(true).append(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let file = options
            .open(&path)
            .map_err(|_| refused("cannot open campaign ledger"))?;
        file.try_lock()
            .map_err(|_| refused("campaign ledger is already locked or cannot be locked"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            let opened = file
                .metadata()
                .map_err(|_| refused("cannot inspect campaign file"))?;
            let named = std::fs::symlink_metadata(&path)
                .map_err(|_| refused("campaign file disappeared"))?;
            let directory = std::fs::metadata(&parent)
                .map_err(|_| refused("cannot inspect campaign parent"))?;
            if !opened.is_file()
                || !named.is_file()
                || opened.mode() & 0o077 != 0
                || opened.nlink() != 1
                || opened.uid() != directory.uid()
                || opened.dev() != named.dev()
                || opened.ino() != named.ino()
            {
                return Err(refused(
                    "campaign ledger must be an owner-only regular file (0600), without links or replacement",
                ));
            }
        }
        let mut state = State {
            file,
            cap,
            committed: 0,
            next_attempt: 1,
            pending: BTreeMap::new(),
            halted: false,
        };
        let entries = BufReader::new(&state.file)
            .lines()
            .map(|line| {
                let line = line.map_err(|_| refused("cannot read campaign ledger"))?;
                serde_json::from_str::<Entry>(&line)
                    .map_err(|_| refused("invalid or torn campaign ledger"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if entries.is_empty() {
            state.append(&Entry::Policy {
                version: 1,
                cap_micro_usd: cap,
                endpoint: config.provider.base_url.clone(),
                pricing: pricing.clone(),
            })?;
        } else {
            for (index, entry) in entries.into_iter().enumerate() {
                match entry {
                    Entry::Policy {
                        version: 1,
                        cap_micro_usd,
                        endpoint,
                        pricing: stored,
                    } if index == 0
                        && cap_micro_usd == cap
                        && endpoint == config.provider.base_url
                        && stored == pricing => {}
                    Entry::Reserved {
                        attempt,
                        reserved_micro_usd,
                        ..
                    } if index > 0
                        && attempt == state.next_attempt
                        && reserved_micro_usd == per_attempt =>
                    {
                        state.add_reservation(attempt, reserved_micro_usd)?;
                    }
                    Entry::Settled {
                        attempt,
                        charged_micro_usd,
                        input_tokens,
                        output_tokens,
                    } if index > 0
                        && input_tokens <= pricing.max_input_tokens
                        && output_tokens <= u64::from(pricing.max_output_tokens) =>
                    {
                        state.apply_settlement(attempt, charged_micro_usd)?;
                    }
                    Entry::Halted if index > 0 => state.halted = true,
                    _ => return Err(refused("campaign ledger policy or sequence mismatch")),
                }
            }
        }
        // Also covers reopening after a previous creation-directory sync failed.
        #[cfg(unix)]
        {
            File::open(&parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| refused("cannot durably open campaign ledger"))?;
        }
        Ok(Self {
            pricing,
            state: Mutex::new(state),
        })
    }

    /// USD committed so far, including all unresolved and in-flight reservations.
    pub fn committed_usd(&self) -> f64 {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .committed as f64
            / MICRO_USD as f64
    }

    pub(crate) fn constrain(&self, request: &mut ModelRequest) -> Result<(), Error> {
        if request.model != self.pricing.model {
            return Err(refused("request model has no campaign price ceiling"));
        }
        request.max_output_tokens = Some(
            request
                .max_output_tokens
                .unwrap_or(self.pricing.max_output_tokens)
                .min(self.pricing.max_output_tokens),
        );
        Ok(())
    }

    pub(crate) fn reserve(
        &self,
        context: &CallContext,
        role: Option<AiRole>,
    ) -> Result<u64, Error> {
        let reserved = ceiling(
            &self.pricing,
            self.pricing.max_input_tokens,
            self.pricing.max_output_tokens.into(),
        )?;
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.halted
            || state
                .committed
                .checked_add(reserved)
                .is_none_or(|total| total > state.cap)
        {
            return Err(refused(
                "aggregate campaign ceiling reached or ledger halted",
            ));
        }
        let attempt = state.next_attempt;
        state.append(&Entry::Reserved {
            attempt,
            reserved_micro_usd: reserved,
            audit_id: context.audit_id.clone(),
            role,
        })?;
        state.add_reservation(attempt, reserved)?;
        Ok(attempt)
    }

    pub(crate) fn settle(&self, attempt: u64, response: &ModelResponse) -> Result<(), Error> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(cost) = response.cost_usd else {
            return Err(refused(
                "provider omitted charge; full attempt reservation retained",
            ));
        };
        let cost = dollars(cost)?;
        let expected = ceiling(&self.pricing, response.input_tokens, response.output_tokens)?;
        if response.model != self.pricing.model
            || response.input_tokens > self.pricing.max_input_tokens
            || response.output_tokens > u64::from(self.pricing.max_output_tokens)
            || cost > expected.saturating_add(1)
            || state
                .pending
                .get(&attempt)
                .is_none_or(|reserved| cost > *reserved || expected > *reserved)
        {
            state.halted = true;
            state.append(&Entry::Halted)?;
            return Err(refused(
                "provider exceeded researched model, token or price ceiling; campaign halted",
            ));
        }
        state.append(&Entry::Settled {
            attempt,
            charged_micro_usd: cost,
            input_tokens: response.input_tokens,
            output_tokens: response.output_tokens,
        })?;
        state.apply_settlement(attempt, cost)
    }
}

impl State {
    fn append(&mut self, entry: &Entry) -> Result<(), Error> {
        if serde_json::to_writer(&mut self.file, entry).is_err()
            || self.file.write_all(b"\n").is_err()
            || self.file.sync_data().is_err()
        {
            self.halted = true;
            return Err(refused(
                "cannot durably write campaign ledger; campaign halted",
            ));
        }
        Ok(())
    }

    fn add_reservation(&mut self, attempt: u64, amount: u64) -> Result<(), Error> {
        self.committed = self
            .committed
            .checked_add(amount)
            .filter(|total| *total <= self.cap)
            .ok_or_else(|| refused("ledger exceeds campaign ceiling"))?;
        self.next_attempt = attempt
            .checked_add(1)
            .ok_or_else(|| refused("campaign attempt overflow"))?;
        self.pending.insert(attempt, amount);
        Ok(())
    }

    fn apply_settlement(&mut self, attempt: u64, cost: u64) -> Result<(), Error> {
        let reserved = self
            .pending
            .remove(&attempt)
            .filter(|reserved| cost <= *reserved)
            .ok_or_else(|| refused("invalid or duplicate campaign settlement"))?;
        self.committed = self.committed - reserved + cost;
        Ok(())
    }
}

fn dollars(value: f64) -> Result<u64, Error> {
    let micros = (value * MICRO_USD as f64).ceil();
    if !value.is_finite() || value < 0.0 || micros >= u64::MAX as f64 {
        return Err(refused("cost must be a finite non-negative dollar amount"));
    }
    Ok(micros as u64)
}

fn ceiling(pricing: &CampaignPricing, input: u64, output: u64) -> Result<u64, Error> {
    input
        .checked_mul(pricing.input_micro_usd_per_million)
        .and_then(|cost| {
            output
                .checked_mul(pricing.output_micro_usd_per_million)
                .and_then(|other| cost.checked_add(other))
        })
        .map(|cost| cost.div_ceil(MICRO_USD))
        .ok_or_else(|| refused("campaign price ceiling overflow"))
}

#[cfg(test)]
#[path = "campaign_tests.rs"]
mod tests;
