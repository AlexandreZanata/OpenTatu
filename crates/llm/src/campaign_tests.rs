//! Offline campaign admission, durability, concurrency and provider-failure tests.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use opentatu_core::{AiRole, Timestamp};

use super::*;
use crate::{FakeClient, Llm, Message, RetryPolicy};

static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

struct Ledger(PathBuf);

impl Ledger {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "opentatu-campaign-{}-{}.jsonl",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt as _;
            builder.mode(0o700);
        }
        builder.create(&directory).unwrap();
        Self(directory.join("campaign.jsonl"))
    }

    fn open(&self, cap: f64) -> CampaignBudget {
        CampaignBudget::open(&self.0, cap, &config()).unwrap()
    }
}

impl Drop for Ledger {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_dir(self.0.parent().unwrap());
    }
}

fn config() -> ModelsConfig {
    ModelsConfig::from_toml(include_str!("../../../config/models.toml")).unwrap()
}

fn context() -> CallContext {
    let at: Timestamp = "2026-11-14T10:00:00-03:00".parse().unwrap();
    CallContext::diagnostic("campaign-test", &at)
}

fn response(cost: Option<f64>) -> ModelResponse {
    ModelResponse {
        content: "{}".into(),
        model: config().campaign.unwrap().model,
        input_tokens: 5_000,
        output_tokens: 1_000,
        reasoning_tokens: 0,
        cost_usd: cost,
        finish_reason: Some("stop".into()),
        duration_ms: 1,
    }
}

#[test]
fn r13_attempts_reserve_the_full_context_and_bounded_output() {
    let file = Ledger::new();
    let campaign = file.open(1.0);
    campaign.reserve(&context(), None).unwrap();
    assert_eq!(campaign.committed_usd(), 0.339322);
    campaign.reserve(&context(), None).unwrap();
    assert!(campaign.reserve(&context(), None).is_err());
    assert_eq!(campaign.committed_usd(), 0.678644);
}

#[test]
fn r13_unresolved_reservations_survive_reopen_and_exclude_other_processes() {
    let file = Ledger::new();
    let campaign = file.open(0.5);
    campaign.reserve(&context(), None).unwrap();
    assert!(CampaignBudget::open(&file.0, 0.5, &config()).is_err());
    drop(campaign);
    let campaign = file.open(0.5);
    assert_eq!(campaign.committed_usd(), 0.339322);
    assert!(campaign.reserve(&context(), None).is_err());
}

#[test]
fn r13_cap_pricing_and_endpoint_cannot_change_on_reopen() {
    let file = Ledger::new();
    drop(file.open(20.0));
    assert!(CampaignBudget::open(&file.0, 21.0, &config()).is_err());
    let mut changed = config();
    changed
        .campaign
        .as_mut()
        .unwrap()
        .input_micro_usd_per_million = 1;
    assert!(CampaignBudget::open(&file.0, 20.0, &changed).is_err());
    let mut changed = config();
    changed.provider.base_url.push_str("/other");
    assert!(CampaignBudget::open(&file.0, 20.0, &changed).is_err());
}

#[test]
fn r13_concurrent_admission_counts_in_flight_calls() {
    let file = Ledger::new();
    let campaign = Arc::new(file.open(1.0));
    let handles: Vec<_> = (0..12)
        .map(|_| {
            let campaign = campaign.clone();
            std::thread::spawn(move || campaign.reserve(&context(), Some(AiRole::Verifier)).is_ok())
        })
        .collect();
    let accepted = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|accepted| *accepted)
        .count();
    assert_eq!(accepted, 2);
    assert!(campaign.committed_usd() <= 1.0);
}

#[test]
fn r13_settlements_round_up_and_release_only_unused_reservations() {
    let file = Ledger::new();
    let campaign = file.open(0.5);
    let id = campaign.reserve(&context(), None).unwrap();
    campaign.settle(id, &response(Some(0.0000011))).unwrap();
    assert_eq!(campaign.committed_usd(), 0.000002);
    drop(campaign);
    let campaign = file.open(0.5);
    assert_eq!(campaign.committed_usd(), 0.000002);
    assert!(campaign.reserve(&context(), None).is_ok());
}

#[test]
fn r13_missing_or_invalid_charge_is_not_free() {
    for cost in [None, Some(f64::NAN), Some(-0.1)] {
        let file = Ledger::new();
        let campaign = file.open(0.5);
        let attempt = campaign.reserve(&context(), None).unwrap();
        assert!(campaign.settle(attempt, &response(cost)).is_err());
        assert_eq!(campaign.committed_usd(), 0.339322);
        assert!(campaign.reserve(&context(), None).is_err());
    }
}

#[test]
fn r13_provider_ceiling_violation_halts_later_calls_and_reopens_halted() {
    let file = Ledger::new();
    let campaign = file.open(20.0);
    let attempt = campaign.reserve(&context(), None).unwrap();
    assert!(campaign.settle(attempt, &response(Some(1.0))).is_err());
    assert!(campaign.reserve(&context(), None).is_err());
    drop(campaign);
    assert!(file.open(20.0).reserve(&context(), None).is_err());
}

#[test]
fn r13_price_drift_below_the_full_reservation_also_halts() {
    let file = Ledger::new();
    let campaign = file.open(20.0);
    let attempt = campaign.reserve(&context(), None).unwrap();
    assert!(campaign.settle(attempt, &response(Some(0.01))).is_err());
    assert!(campaign.reserve(&context(), None).is_err());
}

#[test]
fn r13_fractional_cap_is_rounded_down() {
    let file = Ledger::new();
    let campaign = file.open(0.3393219);
    assert!(campaign.reserve(&context(), None).is_err());
    assert_eq!(campaign.committed_usd(), 0.0);
}

#[cfg(unix)]
#[test]
fn private_ledger_rejects_broad_permissions_and_symlinks() {
    use std::os::unix::fs::{PermissionsExt as _, symlink};
    let file = Ledger::new();
    drop(file.open(20.0));
    std::fs::set_permissions(&file.0, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(CampaignBudget::open(&file.0, 20.0, &config()).is_err());
    std::fs::set_permissions(&file.0, std::fs::Permissions::from_mode(0o600)).unwrap();
    let link = file.0.with_extension("link");
    symlink(&file.0, &link).unwrap();
    assert!(CampaignBudget::open(&link, 20.0, &config()).is_err());
    std::fs::remove_file(link).unwrap();
    let parent = file.0.parent().unwrap();
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(CampaignBudget::open(&file.0, 20.0, &config()).is_err());
}

#[test]
fn r13_torn_ledger_is_never_silently_reset() {
    let file = Ledger::new();
    drop(file.open(20.0));
    let mut output = OpenOptions::new().append(true).open(&file.0).unwrap();
    output.write_all(b"{\"event\":\"reserved\"").unwrap();
    assert!(CampaignBudget::open(&file.0, 20.0, &config()).is_err());
}

#[tokio::test]
async fn r13_every_transport_retry_requires_a_fresh_reservation() {
    let file = Ledger::new();
    let campaign = Arc::new(file.open(0.5));
    let fake = Arc::new(FakeClient::new());
    fake.push_error(Error::Transport("unknown outcome".into()));
    fake.push_response(response(Some(0.01)));
    let llm = Llm::new(fake.clone(), config())
        .with_campaign_budget(campaign.clone())
        .with_retry_policy(RetryPolicy {
            max_retries: 3,
            ..RetryPolicy::none()
        });
    let result = llm
        .generate(&context(), AiRole::Hunter, vec![Message::user("x")], None)
        .await;
    assert!(matches!(result, Err(Error::CampaignBudget(_))));
    assert_eq!(
        fake.requests().len(),
        1,
        "retry was rejected before transport"
    );
    assert_eq!(campaign.committed_usd(), 0.339322);
}

#[tokio::test]
async fn r13_router_bounds_output_and_settles_successful_usage() {
    let file = Ledger::new();
    let campaign = Arc::new(file.open(20.0));
    let fake = Arc::new(FakeClient::new());
    fake.push_response(response(Some(0.001)));
    let llm = Llm::new(fake.clone(), config()).with_campaign_budget(campaign.clone());
    llm.generate(&context(), AiRole::Hunter, vec![Message::user("x")], None)
        .await
        .unwrap();
    assert_eq!(fake.requests()[0].max_output_tokens, Some(32_768));
    assert_eq!(campaign.committed_usd(), 0.001);
    assert!(
        llm.generate(&context(), AiRole::Reporter, vec![], None)
            .await
            .is_err()
    );
    assert_eq!(
        fake.requests().len(),
        1,
        "unpriced model never reaches transport"
    );
}

#[tokio::test]
async fn r13_malformed_provider_response_retains_reservation() {
    let file = Ledger::new();
    let campaign = Arc::new(file.open(0.5));
    let fake = Arc::new(FakeClient::new());
    fake.push_error(Error::Provider("malformed successful response".into()));
    let llm = Llm::new(fake.clone(), config()).with_campaign_budget(campaign.clone());
    assert!(
        llm.generate(&context(), AiRole::Hunter, vec![], None)
            .await
            .is_err()
    );
    assert_eq!(campaign.committed_usd(), 0.339322);
    assert!(
        llm.generate(&context(), AiRole::Hunter, vec![], None)
            .await
            .is_err()
    );
    assert_eq!(fake.requests().len(), 1);
}
