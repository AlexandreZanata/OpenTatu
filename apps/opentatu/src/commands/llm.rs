//! `opentatu llm ...`

use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use opentatu_core::{Clock, SystemClock};
use opentatu_llm::{CallContext, Llm, ModelsConfig, OpenCodeGoClient, RetryPolicy};

use crate::output;
use crate::usage::ServicesUsageRecorder;

/// `opentatu llm ping [--model <id>]`: one minimal call per model; prints model, latency,
/// tokens and cost, and shows that the call was recorded in `llm_usage` (spec §D.2, phase 0).
pub async fn ping(
    data_dir: &Path,
    models_config: &Path,
    model: Option<String>,
) -> anyhow::Result<()> {
    let config = ModelsConfig::load(models_config)?;
    let problems = config.validate();
    anyhow::ensure!(
        problems.is_empty(),
        "{} is not ready:\n  - {}",
        models_config.display(),
        problems.join("\n  - ")
    );
    let api_key = crate::config::api_key(&config.provider.api_key_env)?;
    let client = OpenCodeGoClient::new(&config.provider, api_key)?;
    let services = super::services(data_dir).await?;
    let models: Vec<String> = match model {
        Some(model) => vec![model],
        None => config
            .role_models()
            .into_iter()
            .map(str::to_owned)
            .collect(),
    };
    // Interactive diagnostic: fail fast instead of waiting on rate-limit backoff.
    let llm = Llm::new(Arc::new(client), config)
        .with_recorder(Arc::new(ServicesUsageRecorder::new(services.clone())))
        .with_retry_policy(RetryPolicy::none());

    for model in models {
        let context = CallContext::diagnostic("ping", &SystemClock.now());
        let response = llm
            .ping(&context, &model)
            .await
            .with_context(|| format!("ping to `{model}` failed"))?;
        let cost = response
            .cost_usd
            .map_or_else(|| "n/a".to_owned(), |c| format!("{c:.6}"));
        output::line(&format!(
            "model={} latency_ms={} input_tokens={} output_tokens={} reasoning_tokens={} \
             cost_usd={cost} answer={}",
            response.model,
            response.duration_ms,
            response.input_tokens,
            response.output_tokens,
            response.reasoning_tokens,
            response.content.trim()
        ));
        let month = services.llm_usage_this_month(&response.model).await?;
        output::line(&format!(
            "  recorded in llm_usage; this month: calls={} input_tokens={} output_tokens={} \
             cost_usd={:.6}",
            month.calls, month.input_tokens, month.output_tokens, month.cost_usd
        ));
    }
    Ok(())
}
