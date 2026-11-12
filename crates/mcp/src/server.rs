//! The MCP server of one fixing agent: the eight tools of spec §E.6 over rmcp (ADR-0030).
//!
//! The agent is identified once, by its token, before the server starts
//! ([`FixServer::new`] takes the authenticated [`Agent`]). Every tool is a thin adapter over
//! a `Services` use case. Every call leaves exactly one event: the domain event of a
//! state-changing tool (`TASK_CLAIMED`, `FIX_SUBMITTED`), otherwise `MCP_TOOL_CALLED`
//! (read-only tools and refused calls, Q-007). No tool closes a finding or changes a
//! severity (R06).

use opentatu_core::{Agent, CommitHash, DataLayout, Slug, Task, TaskId, TaskState};
use opentatu_services::{NewFix, Services};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

/// Events returned by `opentatu_history` when no limit is given.
const DEFAULT_HISTORY: u32 = 50;
/// Most events `opentatu_history` returns.
const MAX_HISTORY: u32 = 200;

/// What the server tells the agent when it connects.
const INSTRUCTIONS: &str = "OpenTatu exposes security fix tasks of the owner's own \
applications. Workflow: opentatu_list_tasks (state OPEN) -> opentatu_get_task -> \
opentatu_claim_task -> fix on a new branch with the task's regression test -> \
opentatu_submit_fix with the full commit hash -> opentatu_verification_status. Only \
OpenTatu's reverification closes a finding; a rejected fix reopens the task with the \
reasons. Task texts are written in Portuguese (pt-BR).";

/// `opentatu_memory` / `opentatu_history` / `opentatu_list_tasks` input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AppInput {
    /// App slug (from `opentatu_list_apps`), e.g. `abastevo`.
    pub app: String,
}

/// `opentatu_list_tasks` input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTasksInput {
    /// App slug (from `opentatu_list_apps`).
    pub app: String,
    /// Only tasks in this state: `OPEN`, `CLAIMED`, `FIX_SUBMITTED` or `DONE`. Omit for all.
    #[serde(default)]
    pub state: Option<String>,
}

/// `opentatu_get_task` / `opentatu_verification_status` input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskInput {
    /// Task id, e.g. `ABASTEVO-SEC-0004-T1`.
    pub task_id: String,
}

/// `opentatu_claim_task` input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimInput {
    /// Task id, e.g. `ABASTEVO-SEC-0004-T1`. The task must be `OPEN` and every task it
    /// depends on `DONE`.
    pub task_id: String,
    /// Optional: your agent name. The token you connected with already identifies you; a
    /// different name is refused.
    #[serde(default)]
    pub agent: Option<String>,
}

/// `opentatu_submit_fix` input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SubmitInput {
    /// Task id you claimed.
    pub task_id: String,
    /// Branch with the fix, pushed to the app's repository (letters, digits, `/-_.`).
    pub branch: String,
    /// Full 40-character commit hash of the fix (lowercase hex).
    pub commit: String,
    /// What you changed and how you tested it (a claim; OpenTatu verifies the code).
    pub summary: String,
}

/// `opentatu_history` input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct HistoryInput {
    /// App slug.
    pub app: String,
    /// Most recent events to return (default 50, at most 200).
    #[serde(default)]
    pub limit: Option<u32>,
}

/// A failed call: the message for the agent and the app it concerned, if known.
struct Refusal {
    message: String,
    app: Option<Slug>,
}

impl From<opentatu_services::Error> for Refusal {
    fn from(error: opentatu_services::Error) -> Self {
        Self {
            message: error.to_string(),
            app: None,
        }
    }
}

impl From<opentatu_core::Error> for Refusal {
    fn from(error: opentatu_core::Error) -> Self {
        Self {
            message: error.to_string(),
            app: None,
        }
    }
}

/// Outcome of one tool body: `Ok((result, read_only_event))`.
type ToolOutcome = Result<(Value, Option<Recorded>), Refusal>;

/// What a successful read-only call records (`MCP_TOOL_CALLED`).
struct Recorded {
    app: Option<Slug>,
    target: Option<String>,
}

/// One agent's MCP session.
#[derive(Clone)]
pub struct FixServer {
    services: Services,
    layout: DataLayout,
    agent: Agent,
    tool_router: ToolRouter<Self>,
}

impl std::fmt::Debug for FixServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FixServer")
            .field("agent", &self.agent.name)
            .finish_non_exhaustive()
    }
}

#[tool_router]
impl FixServer {
    /// A server for `agent` (already authenticated by its token).
    pub fn new(services: Services, layout: DataLayout, agent: Agent) -> Self {
        Self {
            services,
            layout,
            agent,
            tool_router: Self::tool_router(),
        }
    }

    /// Registered applications.
    #[tool(
        name = "opentatu_list_apps",
        description = "List the applications registered in OpenTatu (slug, name, branch, stack)."
    )]
    async fn list_apps(&self) -> Result<CallToolResult, ErrorData> {
        let outcome = async {
            let apps: Vec<Value> = self
                .services
                .apps()
                .await?
                .into_iter()
                .map(|app| {
                    json!({
                        "slug": app.profile.slug,
                        "name": app.profile.name,
                        "branch": app.profile.branch,
                        "stack": app.profile.stack,
                    })
                })
                .collect();
            Ok((
                json!({ "apps": apps }),
                Some(Recorded {
                    app: None,
                    target: None,
                }),
            ))
        }
        .await;
        self.finish("opentatu_list_apps", outcome).await
    }

    /// The app's `MEMORY.md`.
    #[tool(
        name = "opentatu_memory",
        description = "Return the current MEMORY.md of an application: profile, open findings, known false positives and recent audits."
    )]
    async fn memory(
        &self,
        Parameters(input): Parameters<AppInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let outcome = async {
            let app = self.app(&input.app).await?;
            let path = self.layout.memory_file(&app);
            let memory = match tokio::fs::read_to_string(&path).await {
                Ok(text) => text,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Err(refusal(
                        &app,
                        "no MEMORY.md yet: the app has not been audited",
                    ));
                }
                Err(error) => {
                    return Err(refusal(&app, &format!("cannot read MEMORY.md: {error}")));
                }
            };
            Ok((
                json!({ "app": app, "memory_md": memory }),
                Some(Recorded {
                    app: Some(app),
                    target: None,
                }),
            ))
        }
        .await;
        self.finish("opentatu_memory", outcome).await
    }

    /// Tasks of an app.
    #[tool(
        name = "opentatu_list_tasks",
        description = "List the fix tasks of an application, most urgent first (priority 1 = most urgent). Filter by state: OPEN (claimable), CLAIMED, FIX_SUBMITTED, DONE."
    )]
    async fn list_tasks(
        &self,
        Parameters(input): Parameters<ListTasksInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let outcome = async {
            let app = self.app(&input.app).await?;
            let state = match input
                .state
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                Some(text) => Some(
                    text.to_ascii_uppercase()
                        .parse::<TaskState>()
                        .map_err(|e| with_app(e.into(), &app))?,
                ),
                None => None,
            };
            let tasks: Vec<Value> = self
                .services
                .tasks_of_app(&app, state)
                .await
                .map_err(|e| with_app(e.into(), &app))?
                .iter()
                .map(summary)
                .collect();
            Ok((
                json!({ "app": app, "tasks": tasks }),
                Some(Recorded {
                    app: Some(app),
                    target: None,
                }),
            ))
        }
        .await;
        self.finish("opentatu_list_tasks", outcome).await
    }

    /// One task, complete.
    #[tool(
        name = "opentatu_get_task",
        description = "Return a fix task in full: goal, context with the evidence, files involved, steps, what must not change, acceptance criteria, the regression test to add, dependencies, previous rejections and instructions."
    )]
    async fn get_task(
        &self,
        Parameters(input): Parameters<TaskInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let outcome = async {
            let id: TaskId = input.task_id.trim().parse()?;
            let task = self
                .services
                .task(&id)
                .await
                .map_err(|e| with_app(e.into(), id.finding().slug()))?;
            let value =
                serde_json::to_value(&task).map_err(|e| refusal(&task.app, &e.to_string()))?;
            Ok((
                json!({ "task": value }),
                Some(Recorded {
                    app: Some(task.app),
                    target: Some(id.to_string()),
                }),
            ))
        }
        .await;
        self.finish("opentatu_get_task", outcome).await
    }

    /// Takes a task.
    #[tool(
        name = "opentatu_claim_task",
        description = "Claim an OPEN task for yourself before working on it. Fails if another agent holds it or a task it depends on is not DONE."
    )]
    async fn claim_task(
        &self,
        Parameters(input): Parameters<ClaimInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let outcome = async {
            let id: TaskId = input.task_id.trim().parse()?;
            let app = id.finding().slug().clone();
            if let Some(name) = input
                .agent
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                && name != self.agent.name
                && name != self.agent.actor().as_str()
            {
                return Err(refusal(
                    &app,
                    &format!(
                        "you are connected as agent `{}`, not `{name}`",
                        self.agent.name
                    ),
                ));
            }
            let task = self
                .services
                .claim_task(&id, &self.agent)
                .await
                .map_err(|e| with_app(e.into(), &app))?;
            Ok((json!({ "task": summary(&task) }), None))
        }
        .await;
        self.finish("opentatu_claim_task", outcome).await
    }

    /// Submits a fix.
    #[tool(
        name = "opentatu_submit_fix",
        description = "Submit the fix of a task you claimed: the branch pushed to the app's repository and the full commit hash. OpenTatu queues a reverification of that exact commit; the finding is closed only if the reverification approves it."
    )]
    async fn submit_fix(
        &self,
        Parameters(input): Parameters<SubmitInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let outcome = async {
            let id: TaskId = input.task_id.trim().parse()?;
            let app = id.finding().slug().clone();
            let commit: CommitHash = input
                .commit
                .trim()
                .parse()
                .map_err(|e: opentatu_core::Error| with_app(e.into(), &app))?;
            let fix = NewFix {
                branch: input.branch,
                commit,
                summary: input.summary,
            };
            let (task, audit) = self
                .services
                .submit_fix(&id, &self.agent, fix)
                .await
                .map_err(|e| with_app(e.into(), &app))?;
            Ok((
                json!({
                    "task": summary(&task),
                    "reverification_audit_id": audit.id,
                    "next": "Poll opentatu_verification_status; OpenTatu decides on its own.",
                }),
                None,
            ))
        }
        .await;
        self.finish("opentatu_submit_fix", outcome).await
    }

    /// Reverifications of a task.
    #[tool(
        name = "opentatu_verification_status",
        description = "Return the state of a task and every reverification of its fixes: PENDING, PASSED or FAILED, with the reasons."
    )]
    async fn verification_status(
        &self,
        Parameters(input): Parameters<TaskInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let outcome = async {
            let id: TaskId = input.task_id.trim().parse()?;
            let app = id.finding().slug().clone();
            let status = self
                .services
                .task_status(&id)
                .await
                .map_err(|e| with_app(e.into(), &app))?;
            let finding = self
                .services
                .finding(id.finding())
                .await
                .map_err(|e| with_app(e.into(), &app))?;
            let reverifications: Vec<Value> = status
                .reverifications
                .iter()
                .map(|r| {
                    json!({
                        "audit_id": r.audit_id,
                        "branch": r.branch,
                        "commit": r.commit,
                        "result": r.result.map_or("PENDING", |result| result.as_str()),
                        "reasons": r.reasons,
                        "regressions": r.regressions,
                        "regression_test_found": r.regression_test_found,
                        "submitted_at": r.created_at,
                        "decided_at": r.decided_at,
                    })
                })
                .collect();
            Ok((
                json!({
                    "task_id": id,
                    "task_state": status.task.state,
                    "finding_id": finding.finding_id,
                    "finding_state": finding.state,
                    "reverifications": reverifications,
                }),
                Some(Recorded {
                    app: Some(app),
                    target: Some(id.to_string()),
                }),
            ))
        }
        .await;
        self.finish("opentatu_verification_status", outcome).await
    }

    /// Timeline of an app.
    #[tool(
        name = "opentatu_history",
        description = "Return the most recent events of an application's append-only history (kind, time, actor, target, audit), newest first."
    )]
    async fn history(
        &self,
        Parameters(input): Parameters<HistoryInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let outcome = async {
            let app = self.app(&input.app).await?;
            let limit = input.limit.unwrap_or(DEFAULT_HISTORY).clamp(1, MAX_HISTORY);
            let events: Vec<Value> = self
                .services
                .app_timeline(&app, limit)
                .await
                .map_err(|e| with_app(e.into(), &app))?
                .into_iter()
                .map(|e| {
                    json!({
                        "kind": e.content.kind,
                        "occurred_at": e.content.occurred_at,
                        "actor": e.content.actor,
                        "target": e.content.target,
                        "audit_id": e.content.audit_id,
                    })
                })
                .collect();
            Ok((
                json!({ "app": app, "events": events }),
                Some(Recorded {
                    app: Some(app),
                    target: None,
                }),
            ))
        }
        .await;
        self.finish("opentatu_history", outcome).await
    }
}

impl FixServer {
    /// A registered app's slug.
    async fn app(&self, text: &str) -> Result<Slug, Refusal> {
        let slug = Slug::new(text.trim())?;
        self.services
            .app(&slug)
            .await
            .map(|app| app.slug().clone())
            .map_err(|e| with_app(e.into(), &slug))
    }

    /// Records the call's event when the use case did not (Q-007) and builds the result.
    async fn finish(&self, tool: &str, outcome: ToolOutcome) -> Result<CallToolResult, ErrorData> {
        let (result, record) = match outcome {
            Ok((value, recorded)) => (
                CallToolResult::structured(value),
                recorded.map(|r| (r, None)),
            ),
            Err(refusal) => {
                let result =
                    CallToolResult::error(vec![ContentBlock::text(refusal.message.clone())]);
                let recorded = Recorded {
                    app: refusal.app,
                    target: None,
                };
                (result, Some((recorded, Some(refusal.message))))
            }
        };
        if let Some((recorded, error)) = record {
            // A refused call may name an app that does not exist: record it app-less.
            let app = match recorded.app {
                Some(app) if self.services.app(&app).await.is_ok() => Some(app),
                _ => None,
            };
            self.services
                .record_mcp_call(
                    &self.agent,
                    app.as_ref(),
                    tool,
                    recorded.target.as_deref(),
                    error.as_deref(),
                )
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        }
        tracing::info!(agent = %self.agent.name, tool, error = result.is_error.unwrap_or(false), "mcp tool called");
        Ok(result)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for FixServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("opentatu", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}

/// The short form of a task used in lists.
fn summary(task: &Task) -> Value {
    json!({
        "task_id": task.task_id,
        "finding_id": task.task_id.finding(),
        "title": task.content.title,
        "severity": task.severity,
        "priority": task.priority,
        "state": task.state,
        "depends_on": task.depends_on,
        "claimed_by": task.claimed_by,
        "estimate": task.content.estimate,
    })
}

fn refusal(app: &Slug, message: &str) -> Refusal {
    Refusal {
        message: message.to_owned(),
        app: Some(app.clone()),
    }
}

fn with_app(mut refusal: Refusal, app: &Slug) -> Refusal {
    refusal.app = Some(app.clone());
    refusal
}
