//! The MCP server spoken to as an agent would: newline-delimited JSON-RPC over a pipe, with
//! the older `initialize` handshake and with 2026-07-28 request metadata.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use opentatu_core::{
    Actor, AgentToken, AuditKind, CodeLocation, Confidence, DataLayout, EventKind, EvidenceItem,
    EvidenceStep, FindingState, Fingerprint, FingerprintInput, FixedClock, HypothesisContent,
    NewApp, Severity, SeverityAdjustments, SkepticReview, Slug, TaskContent, TaskState, Verdict,
    Verification,
};
use opentatu_services::{DedupOutcome, NewAudit, NewHypothesis, PlannedTask, Services};
use serde_json::{Value, json};
use tokio::io::{
    AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, Lines, ReadHalf, WriteHalf,
};

const MODERN: &str = "2026-07-28";

/// Services with app `demo`, an upheld finding DEMO-SEC-0001 and its task T1.
async fn planned() -> Services {
    let clock = Arc::new(FixedClock::at("2026-11-12T09:00:00-03:00").unwrap());
    let services = Services::in_memory(clock).await.unwrap();
    let slug = Slug::new("demo").unwrap();
    services
        .register_app(NewApp {
            slug: slug.clone(),
            name: "Demo".into(),
            repo_url: "/tmp/demo".into(),
            branch: "main".into(),
            stack: vec!["Go".into()],
            sensitive_data: vec![],
            lab_url: None,
        })
        .await
        .unwrap();
    services
        .create_audit(NewAudit {
            app: slug.clone(),
            kind: AuditKind::Full,
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            branch: None,
            requested_by: Actor::Owner,
        })
        .await
        .unwrap();
    let audit = services.start_next_audit().await.unwrap().unwrap();
    let content = HypothesisContent {
        title: "Pedido de outro usuário".into(),
        category: "Controle de acesso".into(),
        cwe: "CWE-639".into(),
        asvs: vec![],
        entry_point: None,
        location: CodeLocation {
            file: "api/orders.go".into(),
            lines: "3".into(),
            symbol: "GetOrder".into(),
        },
        reasoning: "x".into(),
        impact_scenario: "x".into(),
        required_conditions: vec![],
        how_to_verify: "x".into(),
        potential_severity: Severity::High,
        confidence: Confidence::High,
        origin: "manual_analysis".into(),
        possible_regression_of: None,
    };
    let hypothesis = services
        .record_hypothesis(
            &audit,
            NewHypothesis {
                content,
                fingerprint: Fingerprint::compute(&FingerprintInput {
                    cwe: "CWE-639",
                    file: "api/orders.go",
                    symbol: "GetOrder",
                    snippet: "load(id)",
                }),
                model: "m".into(),
                prompt_version: "hunter@2.1.0".into(),
                system_notes: vec![],
            },
            &DedupOutcome::New {
                possibly_same_as: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
    let verification = Verification {
        lab_run_number: None,
        verdict: Verdict::Confirmed,
        justification: "x".into(),
        evidence: vec![EvidenceItem {
            evidence_id: None,
            kind: "code_flow".into(),
            description: "x".into(),
            steps: vec![EvidenceStep {
                file: "api/orders.go".into(),
                line: "3".into(),
                what_happens: "x".into(),
            }],
        }],
        mitigations_found: vec![],
        regression_test: None,
        missing_to_conclude: String::new(),
        model: "m".into(),
        prompt_version: "verifier@1.2.0".into(),
        system_notes: vec![],
    };
    let (_, finding) = services
        .record_verification(&hypothesis.hypothesis_id, verification)
        .await
        .unwrap();
    let finding = finding.unwrap();
    services
        .record_review(
            &finding.finding_id,
            SkepticReview {
                decision: FindingState::Upheld,
                base_severity: Severity::High,
                adjustments: SeverityAdjustments::default(),
                final_severity: Severity::High,
                arguments_considered: vec![],
                justification: "x".into(),
                model: "m".into(),
                prompt_version: "skeptic@2.0.0".into(),
            },
        )
        .await
        .unwrap();
    services
        .record_tasks(
            &audit.id,
            vec![PlannedTask {
                reference: "T1".into(),
                finding_id: finding.finding_id,
                content: TaskContent {
                    title: "Verificar o dono do pedido".into(),
                    goal: "x".into(),
                    context: "x".into(),
                    files_involved: vec!["api/orders.go".into()],
                    steps: vec![],
                    constraints: vec![],
                    acceptance_criteria: vec![],
                    regression_test: None,
                    estimate: "S".into(),
                    instructions_for_agent: "x".into(),
                },
                depends_on: vec![],
                model: "m".into(),
                prompt_version: "planner@2.0.0".into(),
            }],
        )
        .await
        .unwrap();
    services
}

/// A connected client: writes requests, reads one JSON message per line.
struct Client {
    writer: WriteHalf<DuplexStream>,
    lines: Lines<BufReader<ReadHalf<DuplexStream>>>,
    next_id: u64,
    modern: bool,
}

impl Client {
    async fn start(services: &Services, token: &AgentToken, modern: bool) -> Self {
        let (client, server) = tokio::io::duplex(1 << 20);
        let (services, token) = (services.clone(), token.clone());
        let layout = DataLayout::new(std::env::temp_dir().join("opentatu-mcp-test"));
        tokio::spawn(async move {
            let _ = opentatu_mcp::serve(services, layout, &token, tokio::io::split(server)).await;
        });
        let (read, writer) = tokio::io::split(client);
        let mut client = Self {
            writer,
            lines: BufReader::new(read).lines(),
            next_id: 0,
            modern,
        };
        if !modern {
            let init = client
                .request(
                    "initialize",
                    json!({
                        "protocolVersion": "2025-11-25",
                        "capabilities": {},
                        "clientInfo": { "name": "test-agent", "version": "1.0.0" }
                    }),
                )
                .await;
            assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
            assert_eq!(init["result"]["serverInfo"]["name"], "opentatu");
            client
                .send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
                .await;
        }
        client
    }

    async fn send(&mut self, message: Value) {
        let mut line = message.to_string();
        line.push('\n');
        self.writer.write_all(line.as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();
    }

    /// Sends a request and returns its response (notifications are skipped).
    async fn request(&mut self, method: &str, mut params: Value) -> Value {
        self.next_id += 1;
        if self.modern {
            params["_meta"] = json!({
                "io.modelcontextprotocol/protocolVersion": MODERN,
                "io.modelcontextprotocol/clientCapabilities": {},
                "io.modelcontextprotocol/clientInfo": { "name": "test-agent", "version": "1.0.0" }
            });
        }
        let id = self.next_id;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .await;
        loop {
            let line = self.lines.next_line().await.unwrap().unwrap();
            let message: Value = serde_json::from_str(&line).unwrap();
            if message["id"] == json!(id) {
                return message;
            }
        }
    }

    /// Calls a tool; returns `(is_error, structuredContent or text)`.
    async fn call(&mut self, tool: &str, arguments: Value) -> (bool, Value) {
        let response = self
            .request(
                "tools/call",
                json!({ "name": tool, "arguments": arguments }),
            )
            .await;
        let result = &response["result"];
        assert!(result.is_object(), "{response}");
        let is_error = result["isError"].as_bool().unwrap_or(false);
        let body = if result["structuredContent"].is_null() {
            result["content"][0]["text"].clone()
        } else {
            result["structuredContent"].clone()
        };
        (is_error, body)
    }
}

async fn mcp_calls(services: &Services) -> usize {
    services
        .app_history(&Slug::new("demo").unwrap())
        .await
        .unwrap()
        .iter()
        .filter(|e| e.content.kind == EventKind::McpToolCalled)
        .count()
}

#[tokio::test]
async fn legacy_handshake_lists_the_eight_tools_and_claims_a_task() {
    let services = planned().await;
    let token = AgentToken::from_random([5; 32]);
    services.register_agent("codex", &token).await.unwrap();
    let mut client = Client::start(&services, &token, false).await;

    let tools = client.request("tools/list", json!({})).await;
    let mut names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "opentatu_claim_task",
            "opentatu_get_task",
            "opentatu_history",
            "opentatu_list_apps",
            "opentatu_list_tasks",
            "opentatu_memory",
            "opentatu_submit_fix",
            "opentatu_verification_status",
        ],
        "spec §E.6"
    );

    let (error, body) = client
        .call(
            "opentatu_list_tasks",
            json!({ "app": "demo", "state": "open" }),
        )
        .await;
    assert!(!error);
    assert_eq!(body["tasks"][0]["task_id"], "DEMO-SEC-0001-T1");
    let (error, body) = client
        .call(
            "opentatu_get_task",
            json!({ "task_id": "DEMO-SEC-0001-T1" }),
        )
        .await;
    assert!(!error);
    assert_eq!(body["task"]["files_involved"][0], "api/orders.go");
    assert_eq!(mcp_calls(&services).await, 2, "Q-007: read-only calls");

    let (error, _) = client
        .call(
            "opentatu_claim_task",
            json!({ "task_id": "DEMO-SEC-0001-T1", "agent": "someone" }),
        )
        .await;
    assert!(error, "the token decides who the agent is");
    let (error, body) = client
        .call(
            "opentatu_claim_task",
            json!({ "task_id": "DEMO-SEC-0001-T1" }),
        )
        .await;
    assert!(!error, "{body}");
    assert_eq!(body["task"]["state"], "CLAIMED");
    assert_eq!(body["task"]["claimed_by"], "agent:codex");
    assert_eq!(
        mcp_calls(&services).await,
        3,
        "a claim records TASK_CLAIMED, a refused claim MCP_TOOL_CALLED"
    );
}

#[tokio::test]
async fn modern_requests_submit_a_fix_that_only_reverification_can_close() {
    let services = planned().await;
    let token = AgentToken::from_random([6; 32]);
    services.register_agent("claude", &token).await.unwrap();
    let mut client = Client::start(&services, &token, true).await;

    let discover = client.request("server/discover", json!({})).await;
    assert!(
        discover["result"]["supportedVersions"]
            .as_array()
            .unwrap()
            .contains(&json!(MODERN)),
        "{discover}"
    );
    let (error, _) = client
        .call(
            "opentatu_claim_task",
            json!({ "task_id": "DEMO-SEC-0001-T1" }),
        )
        .await;
    assert!(!error);
    let (error, body) = client
        .call(
            "opentatu_submit_fix",
            json!({
                "task_id": "DEMO-SEC-0001-T1",
                "branch": "fix/orders-owner",
                "commit": "9f2a5df92ace8d8068e57362a7308e8b9d031b3a",
                "summary": "Compara o dono do pedido.",
            }),
        )
        .await;
    assert!(!error, "{body}");
    assert_eq!(body["task"]["state"], "FIX_SUBMITTED");
    let audit = body["reverification_audit_id"].as_str().unwrap().to_owned();

    let (_, status) = client
        .call(
            "opentatu_verification_status",
            json!({ "task_id": "DEMO-SEC-0001-T1" }),
        )
        .await;
    assert_eq!(status["reverifications"][0]["result"], "PENDING");
    assert_eq!(status["finding_state"], "FIX_SUBMITTED", "R06");
    let queued = services.audit(&audit.parse().unwrap()).await.unwrap();
    assert_eq!(queued.kind, AuditKind::Reverification);

    let (error, message) = client
        .call(
            "opentatu_get_task",
            json!({ "task_id": "DEMO-SEC-9999-T1" }),
        )
        .await;
    assert!(error, "{message}");
    let (error, _) = client
        .call("opentatu_memory", json!({ "app": "nope" }))
        .await;
    assert!(error);
    let task = services
        .task(&"DEMO-SEC-0001-T1".parse().unwrap())
        .await
        .unwrap();
    assert_eq!(task.state, TaskState::FixSubmitted);
}

#[tokio::test]
async fn r16_an_unknown_token_gets_no_session() {
    let services = planned().await;
    let (_client, server) = tokio::io::duplex(1024);
    let result = opentatu_mcp::serve(
        services,
        DataLayout::new(std::env::temp_dir()),
        &AgentToken::from_random([1; 32]),
        tokio::io::split(server),
    )
    .await;
    let message = result.unwrap_err().to_string();
    assert!(message.contains("unknown or revoked"), "{message}");
    assert!(!message.contains("0101"), "the token is never echoed");
}
