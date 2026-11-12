//! Scripted client for tests. The single test double for LLM calls: reuse it, do not
//! write new fakes.

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};

use crate::{Error, LlmClient, ModelRequest, ModelResponse};

/// Returns scripted answers in order and records every request it receives.
#[derive(Debug, Default)]
pub struct FakeClient {
    answers: Mutex<VecDeque<Result<String, Error>>>,
    requests: Mutex<Vec<ModelRequest>>,
}

impl FakeClient {
    /// Empty client (every call fails with [`Error::FakeExhausted`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues a successful answer with `content`.
    pub fn push_answer(&self, content: impl Into<String>) {
        self.answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(Ok(content.into()));
    }

    /// Queues an error.
    pub fn push_error(&self, error: Error) {
        self.answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(Err(error));
    }

    /// Requests received so far, in order.
    pub fn requests(&self) -> Vec<ModelRequest> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[async_trait::async_trait]
impl LlmClient for FakeClient {
    async fn complete(&self, request: ModelRequest) -> Result<ModelResponse, Error> {
        let model = request.model.clone();
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request);
        let answer = self
            .answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .unwrap_or(Err(Error::FakeExhausted))?;
        let output_tokens = u64::try_from(answer.len()).unwrap_or(u64::MAX);
        Ok(ModelResponse {
            content: answer,
            model,
            input_tokens: 0,
            output_tokens,
            reasoning_tokens: 0,
            cost_usd: None,
            finish_reason: Some("stop".into()),
            duration_ms: 0,
        })
    }
}
