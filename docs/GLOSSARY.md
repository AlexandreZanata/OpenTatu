# Glossary

The project's ubiquitous language. Use the **Term** in code, data formats and docs
(ADR-0001). The "Spec term (pt-BR)" column records the name used in the original
Portuguese document; the words in backticks are **forbidden** everywhere except string
literals, HTML templates and prompt files (owner-facing pt-BR text). `cargo xtask glossary`
enforces it by parsing the table below — keep its shape: one row per term, forbidden words
in backticks in the second column.

## Terms

| Term | Spec term (pt-BR) | Definition | In code |
|---|---|---|---|
| **App** | Aplicação (`aplicacao`, `aplicacoes`) | A registered application owned by the owner (R01) | `apps` table; key `app` |
| **Slug** | slug | Lowercase identifier of an app, e.g. `abastevo` | `Slug` |
| **Snapshot** | Snapshot | Exact state of the analyzed code: one full commit hash (R02) | `snapshots` table |
| **Audit** | Auditoria (`auditoria`, `auditorias`) | One investigation run with ID, kind, status, budget and cost | `AuditId`, `audits` table |
| **Audit kind** | tipos (`completa`) | `FULL`, `INCREMENTAL`, `REVERIFICATION`, `PR` (§A.5) | `AuditKind` |
| **Audit status** | status (`criada`, `pausada`, `parcial`, `concluida`, `falhou`) | `CREATED`, `RUNNING`, `PAUSED`, `PARTIAL`, `COMPLETED`, `FAILED` (Q-002) | `AuditStatus` |
| **Reverification** | Reverificação (`reverificacao`) | Audit kind that checks a submitted fix | `AuditKind::Reverification` |
| **Security map** | Mapa (`mapa`, `mapas`) | Versioned security model of an app: components, entry points, boundaries | `MapId`; `maps` table (phase 3) |
| **Entry point** | Ponto de entrada (`ponto`, `pontos`) | Route, handler, job or webhook receiving external input; ids `EP-001` | map key `entry_points` |
| **Trust boundary** | Fronteira de confiança (`fronteira`, `fronteiras`) | Where external data enters the app | map key `trust_boundaries` |
| **Requirement** | Requisito (`requisito`, `requisitos`) | An OWASP ASVS item applicable to the app | `config/asvs/` (phase 3) |
| **Coverage** | Cobertura (`cobertura`) | Applicable requirements verified, failed and not tested (R14) | `coverage.json` |
| **Hypothesis** | Hipótese (`hipotese`, `hipoteses`) | Vulnerability suspicion without proof yet | `HypothesisId` |
| **Finding** | Achado (`achado`, `achados`) | Hypothesis confirmed with evidence (R05) | `FindingId` |
| **Finding state** | estados (`descartada`, `inconclusiva`, `rebaixado`, `mantido`, `reprovada`, `fechado`) | `HYPOTHESIS` … `CLOSED`, `RISK_ACCEPTED`, `REGRESSION` (§A.6) | `FindingState` |
| **Evidence** | Evidência (`evidencia`, `evidencias`) | Proof supporting a finding (§C.9) | `EvidenceId` |
| **Task** | Tarefa (`tarefa`, `tarefas`) | Unit of fix work for a fixing agent | `TaskId` |
| **Verification** | Verificação (`verificacao`, `verificacoes`) | OpenTatu's check of a fix | `verifications` table (phase 5) |
| **Decision** | Decisão (`decisao`, `decisoes`) | Owner act: accepted risk, false positive, override (R20) | `OWNER_DECISION` |
| **Accepted risk** | Risco aceito (`risco`, `riscos`) | Owner decision with justification and expiry (R07) | `FindingState::RiskAccepted` |
| **False positive** | Falso positivo (`falso`) | Dismissed with reason and fingerprint (R08) | `FALSE_POSITIVE_RECORDED` |
| **Regression** | Regressão (`regressao`, `regressoes`) | Closed finding whose fingerprint reappears (R09) | `FindingState::Regression` |
| **Event** | Evento (`evento`, `eventos`) | Immutable record of anything that happened (R04) | `Event`, `NewEvent`, `EventKind` |
| **Actor** | Ator (`ator`, `atores`) | Who caused an event: `system`, `owner`, an AI role, `agent:<id>` | `Actor` |
| **Owner** | Dono (`dono`) | The single person who owns OpenTatu and the apps | `Actor::Owner` |
| **AI role** | Papel (`papel`, `papeis`) | One of the six investigator roles | `AiRole` |
| **Investigator** | Agente investigador (`investigador`) | The internal set of AI roles (no shell, R12) | `opentatu-investigator` |
| **Cartographer** | Cartógrafo (`cartografo`) | Role that builds the security map (§C.1) | `AiRole::Cartographer` |
| **Hunter** | Caçador (`cacador`) | Role that generates hypotheses (§C.2) | `AiRole::Hunter` |
| **Verifier** | Verificador (`verificador`) | Role that proves or refutes hypotheses (§C.3) | `AiRole::Verifier` |
| **Skeptic** | Revisor cético (`cetico`) | Role that tries to refute findings, final severity (§C.4) | `AiRole::Skeptic` |
| **Planner** | Planejador (`planejador`) | Role that writes fix tasks (§C.5) | `AiRole::Planner` |
| **Reporter** | Relator (`relator`) | Role that writes the audit report (§C.6) | `AiRole::Reporter` |
| **Fixing agent** | Agente corretor (`corretor`, `corretores`) | External coding agent using MCP; never closes findings | `Actor::ExternalAgent` |
| **Scanner** | Executor determinístico | Semgrep, Gitleaks, Trivy, OSV-Scanner (later ZAP) in containers | `Scanner`, `opentatu-scanners` |
| **Severity** | Severidade (`severidade`) | `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, `INFO` (§C.8, R18) | `Severity` |
| **Fingerprint** | Fingerprint | `sha256:` identity of a finding across audits (§B.8) | `Fingerprint` |
| **Budget** | Orçamento (`orcamento`) | Per-audit limits of tokens, calls and minutes (R13) | `[budget]` in `config/models.toml` |
| **Usage** | Consumo (`consumo`) | Tokens and calls actually spent | `llm_usage` (phase 0) |
| **Working memory** | Memória (`memoria`) | `MEMORY.md`, the summary each audit starts from (§B.9) | `opentatu-memory` |
| **History** | Histórico (`historico`) | `history.jsonl`, mirror of the event log | `opentatu-memory` |
| **Report** | Relatório (`relatorio`, `relatorios`) | `report.md` (pt-BR) and its exports | `opentatu-reports` |
| **Gap** | Lacuna (`lacuna`, `lacunas`) | Missing information declared by an AI role | output key `gaps` |
| **Not analyzed** | Não analisado (`analisado`) | Items an AI role skipped (e.g. low budget) | output key `not_analyzed` |
| **Suspicious instruction** | Instrução suspeita (`instrucao`, `instrucoes`, `suspeita`) | Text in analyzed content addressed to the AI (R11) | output key `suspicious_instructions` |
| **Lab** | Laboratório (`laboratorio`) | Isolated environment with fictitious data for dynamic tests (R10) | phase 8 |
| **Sensitive data** | Dados sensíveis (`dados`, `sensiveis`) | Personal, financial, credential or health data of an app | key `sensitive_data` |
| **Dashboard** | Painel (`painel`) | The local web UI | `opentatu-dashboard` |
| **Phase** | Fase (`fase`, `fases`) | A roadmap phase | `docs/ROADMAP.md` |
| **Timestamp** | Data e hora | ISO 8601 instant with offset (§B.3) | `Timestamp`, `Clock` |
| **Use case** | — | One operation of the system, implemented once | `Services` methods |

## Also avoid (English drift, not machine-checked)

| Use | Not |
|---|---|
| Finding | issue, vulnerability (as a type name), vuln, bug |
| Owner | user, admin (there is exactly one owner) |
| App | project, repo, service (an app *has* a repository) |
| Audit | scan, assessment (a scan is one scanner run inside an audit) |
| Task | ticket; "job" is reserved for the worker queue |
| Hypothesis | candidate, suspicion |
| AI role | agent, persona ("agents" are the external fixing agents) |
| Event | log entry (logs are diagnostics, events are the history) |
