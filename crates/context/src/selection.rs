//! The `<code>` and `<config>` blocks: files ranked by how likely they are to define entry
//! points or security controls, packed within an explicit byte budget, with line numbers so
//! roles can cite exact lines (base prompt principle 3).
//!
//! The cartographer runs before any entry point is known, so its code selection is ranked
//! by signals: route/handler declarations, server and socket setup, security vocabulary,
//! entry-file names. Tests, examples and archived code rank last. What does not fit is
//! returned in [`Selection::left_out`] (R14).
//! PENDING(phase-4): per-entry-point selection (handler + middlewares + called functions up
//! to depth N) for the hunter.

use std::fmt::Write as _;

use crate::files::{FileKind, SnapshotFiles};

/// Size limits of one selection (bytes of rendered, line-numbered text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextBudget {
    /// Total bytes of `<code>`.
    pub code_bytes: usize,
    /// Bytes of one file in `<code>` (larger files are cut to excerpts).
    pub code_file_bytes: usize,
    /// Total bytes of `<config>`.
    pub config_bytes: usize,
    /// Bytes of one file in `<config>`.
    pub config_file_bytes: usize,
    /// Lines of `<structure>`.
    pub tree_lines: usize,
}

impl ContextBudget {
    /// The cartographer's budget: about 250K tokens of input in total (DeepSeek V4.1 Flash
    /// reads 1M; about US$0.04 per map at list price, docs/opencode-go.md).
    pub const CARTOGRAPHER: Self = Self {
        code_bytes: 640_000,
        code_file_bytes: 40_000,
        config_bytes: 90_000,
        config_file_bytes: 12_000,
        tree_lines: 3_000,
    };
}

/// One file (or excerpts of it) in a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Excerpt {
    /// Repository-relative path.
    pub path: String,
    /// Line-numbered text.
    pub text: String,
    /// Whether only excerpts of the file are included.
    pub partial: bool,
}

/// The files chosen for a block and the candidates that did not fit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    /// Included files, in rank order.
    pub excerpts: Vec<Excerpt>,
    /// Ranked candidates (with at least one signal) that did not fit the budget.
    pub left_out: Vec<String>,
    /// Bytes of the rendered block.
    pub bytes: usize,
}

impl Selection {
    /// The block contents: each file under a `=== path ===` header.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(self.bytes);
        for excerpt in &self.excerpts {
            out.push_str(&excerpt.text);
        }
        out
    }
}

/// Route, handler, server and socket declarations of common stacks.
const ROUTE_SIGNALS: &[&str] = &[
    // Rust
    ".route(",
    "Router::new",
    "#[get(",
    "#[post(",
    "#[put(",
    "#[delete(",
    "#[patch(",
    "warp::path",
    "HttpServer::new",
    "axum::serve",
    "TcpListener::bind",
    "UdpSocket::bind",
    "accept_async",
    "WebSocketUpgrade",
    "on_upgrade",
    "tonic::",
    "#[tokio::main]",
    // JavaScript / TypeScript
    "app.get(",
    "app.post(",
    "app.put(",
    "app.delete(",
    "router.get(",
    "router.post(",
    "express()",
    "fastify(",
    "export async function GET",
    "export async function POST",
    "WebSocketServer",
    "io.on(",
    "socket.on(",
    "createServer(",
    "@Get(",
    "@Post(",
    "@Controller(",
    "addEventListener(\"message\"",
    // Python
    "@app.route",
    "@app.get",
    "@app.post",
    "@router.get",
    "@router.post",
    "@bp.route",
    "urlpatterns",
    "FastAPI(",
    "Flask(",
    "APIRouter(",
    "HTTPServer(",
    "BaseHTTPRequestHandler",
    "def do_GET",
    "def do_POST",
    "websockets.serve",
    "web.Application(",
    "add_route(",
    // Go
    "http.HandleFunc",
    "HandleFunc(",
    "ListenAndServe",
    "gin.Default",
    "mux.NewRouter",
    // JVM / .NET / PHP / Ruby
    "@RestController",
    "@RequestMapping",
    "@GetMapping",
    "@PostMapping",
    "[HttpGet",
    "[HttpPost",
    "[Route(",
    "MapGet(",
    "MapPost(",
    "Route::get",
    "Route::post",
    "$app->get(",
    "resources :",
];

/// Security vocabulary (matched case-insensitively; each distinct word counts once).
const SECURITY_WORDS: &[&str] = &[
    "auth",
    "login",
    "logout",
    "password",
    "session",
    "token",
    "jwt",
    "cookie",
    "csrf",
    "cors",
    "permission",
    "role",
    "admin",
    "secret",
    "api_key",
    "apikey",
    "signature",
    "hmac",
    "encrypt",
    "decrypt",
    "bcrypt",
    "argon2",
    "oauth",
    "middleware",
    "rate_limit",
    "ratelimit",
    "verify",
    "sanitize",
    "upload",
];

/// File stems that usually start or route an application.
const ENTRY_STEMS: &[&str] = &[
    "main",
    "app",
    "server",
    "index",
    "routes",
    "router",
    "urls",
    "views",
    "api",
    "handlers",
    "handler",
    "controller",
    "controllers",
    "endpoints",
    "ws",
    "websocket",
    "socket",
    "gateway",
    "http",
    "net",
    "network",
    "auth",
    "middleware",
    "session",
];

/// Path words (segments split on `/`, `-`, `_`, `.`) of server-side code and message
/// dispatch. Security words (`auth`, `session`) are scored from the content instead, so
/// client code does not outrank the server.
const SERVER_PATH_WORDS: &[&str] = &[
    "server",
    "api",
    "backend",
    "gateway",
    "admin",
    "identity",
    "ws",
    "websocket",
    "routes",
    "handlers",
    "controllers",
    "msg",
    "message",
    "messages",
    "dispatch",
    "rpc",
    "protocol",
    "proto",
];

/// Path segments of developer tooling (build, packaging, asset scripts): ranked lower.
const TOOLING_SEGMENTS: &[&str] = &["scripts", "tools", "tooling", "devtools", "ci"];

/// Path segments of code that rarely holds production entry points (tests are detected by
/// `files::is_test_path`).
const LOW_VALUE_SEGMENTS: &[&str] = &[
    "examples",
    "example",
    "benches",
    "bench",
    "fixture",
    "fixtures",
    "mock",
    "mocks",
    "archive",
    "deprecated",
    "legacy",
    "docs",
    "generated",
];

/// Lines kept from the top of a file cut to excerpts (imports, module docs).
const HEADER_LINES: usize = 40;
/// Lines kept around each route signal in a file cut to excerpts.
const WINDOW_LINES: usize = 30;
/// Paths listed in `left_out` (the rest is counted).
const MAX_LEFT_OUT: usize = 80;

/// `<code>` for the cartographer.
pub fn cartographer_code(files: &SnapshotFiles, budget: &ContextBudget) -> Selection {
    let candidates = files
        .files()
        .iter()
        .filter(|file| file.kind == FileKind::Source)
        .filter_map(|file| {
            let text = files.read_text(&file.path)?;
            Some((
                file.path.clone(),
                code_score(&file.path, &text.content),
                text.content,
            ))
        })
        .collect();
    pack(candidates, budget.code_bytes, budget.code_file_bytes)
}

/// `<config>` for the cartographer: deployment and container files first, then manifests
/// and CI workflows.
pub fn cartographer_config(files: &SnapshotFiles, budget: &ContextBudget) -> Selection {
    let candidates = files
        .files()
        .iter()
        .filter(|file| file.kind == FileKind::Config)
        .filter_map(|file| {
            let text = files.read_text(&file.path)?;
            Some((file.path.clone(), config_score(&file.path), text.content))
        })
        .collect();
    pack(candidates, budget.config_bytes, budget.config_file_bytes)
}

fn code_score(path: &str, content: &str) -> u32 {
    let routes = ROUTE_SIGNALS
        .iter()
        .filter(|s| content.contains(*s))
        .count();
    let lower = content.to_ascii_lowercase();
    let words = SECURITY_WORDS.iter().filter(|w| lower.contains(*w)).count();
    let stem = path
        .rsplit('/')
        .next()
        .and_then(|name| name.split('.').next())
        .unwrap_or("");
    let server_path = path
        .split(['/', '-', '_', '.'])
        .any(|word| SERVER_PATH_WORDS.contains(&word));
    let mut score = 100 * routes.min(5) + 10 * words.min(10);
    if ENTRY_STEMS.contains(&stem) {
        score += 40;
    }
    if server_path {
        score += 80;
    }
    if is_low_value(path) {
        score /= 20;
    } else if path
        .split('/')
        .any(|segment| TOOLING_SEGMENTS.contains(&segment))
    {
        score /= 4;
    }
    u32::try_from(score).unwrap_or(u32::MAX)
}

fn config_score(path: &str) -> u32 {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    let deployment = name.starts_with("dockerfile")
        || name.starts_with("docker-compose")
        || name.starts_with("compose.")
        || name.starts_with("caddyfile")
        || name.ends_with(".service")
        || name.ends_with(".conf")
        || name.contains(".env.")
        || path.starts_with("deploy");
    let score: u32 = if deployment {
        300
    } else if path.starts_with(".github/workflows/") {
        100
    } else {
        200
    };
    if is_low_value(path) {
        score / 20
    } else {
        score
    }
}

pub(crate) fn is_low_value(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.split('.').next().unwrap_or(name);
    crate::files::is_test_path(path)
        || path
            .split('/')
            .any(|segment| LOW_VALUE_SEGMENTS.contains(&segment))
        || LOW_VALUE_SEGMENTS.contains(&stem)
        || name.contains(".min.")
}

/// Packs ranked `(path, score, content)` candidates into at most `total` bytes.
fn pack(mut candidates: Vec<(String, u32, String)>, total: usize, per_file: usize) -> Selection {
    candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut selection = Selection::default();
    let mut skipped_without_signal = 0usize;
    for (path, score, content) in candidates {
        let excerpt = render_file(&path, &content, per_file, &[]);
        if selection.bytes + excerpt.text.len() <= total {
            selection.bytes += excerpt.text.len();
            selection.excerpts.push(excerpt);
        } else if score > 0 && selection.left_out.len() < MAX_LEFT_OUT {
            selection.left_out.push(path);
        } else {
            skipped_without_signal += 1;
        }
    }
    if skipped_without_signal > 0 {
        selection
            .left_out
            .push(format!("(+{skipped_without_signal} lower-ranked file(s))"));
    }
    selection
}

/// Line-numbered file text, cut to header + windows around route signals when larger
/// than `per_file`.
/// Line-numbered file text; when larger than `per_file`, cut to the header plus windows
/// around route signals and around the `focus` lines (0-based).
pub(crate) fn render_file(path: &str, content: &str, per_file: usize, focus: &[usize]) -> Excerpt {
    let lines: Vec<&str> = content.lines().collect();
    let full = number(path, &lines, &[(0, lines.len())], lines.len(), false);
    if full.len() <= per_file {
        return Excerpt {
            path: path.to_owned(),
            text: full,
            partial: false,
        };
    }
    let mut ranges = vec![(0, HEADER_LINES.min(lines.len()))];
    let mut anchors: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| ROUTE_SIGNALS.iter().any(|s| line.contains(s)))
        .map(|(index, _)| index)
        .collect();
    anchors.extend(focus.iter().copied().filter(|index| *index < lines.len()));
    anchors.sort_unstable();
    anchors.dedup();
    for index in anchors {
        let start = index.saturating_sub(WINDOW_LINES / 3);
        let end = (index + WINDOW_LINES).min(lines.len());
        match ranges.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => ranges.push((start, end)),
        }
    }
    let mut text = number(path, &lines, &ranges, lines.len(), true);
    if text.len() > per_file {
        let cut = text[..per_file].rfind('\n').unwrap_or(0);
        text.truncate(cut + 1);
        text.push_str("     ⋮ (cut to the size budget)\n");
    }
    Excerpt {
        path: path.to_owned(),
        text,
        partial: true,
    }
}

fn number(
    path: &str,
    lines: &[&str],
    ranges: &[(usize, usize)],
    total: usize,
    partial: bool,
) -> String {
    let mut out = String::new();
    let note = if partial { "; excerpts" } else { "" };
    let _ = writeln!(out, "=== {path} ({total} lines{note}) ===");
    let mut previous_end = 0;
    for (start, end) in ranges {
        if *start > previous_end {
            out.push_str("     ⋮\n");
        }
        for (offset, line) in lines[*start..*end].iter().enumerate() {
            let _ = writeln!(out, "{:>5}| {line}", start + offset + 1);
        }
        previous_end = *end;
    }
    if previous_end < total {
        out.push_str("     ⋮\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(entries: &[(&str, String)]) -> (tempfile::TempDir, SnapshotFiles) {
        let dir = tempfile::tempdir().unwrap();
        for (path, contents) in entries {
            let file = dir.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, contents).unwrap();
        }
        let files = SnapshotFiles::scan(dir.path()).unwrap();
        (dir, files)
    }

    #[test]
    fn routes_and_security_code_rank_first_tests_last() {
        let (_dir, files) = snapshot(&[
            (
                "src/server.rs",
                "let app = Router::new().route(\"/login\", post(login));\n".into(),
            ),
            (
                "src/auth.rs",
                "fn verify(token: &str) -> bool { check_session(token) }\n".into(),
            ),
            (
                "src/util.rs",
                "pub fn add(a: i32, b: i32) -> i32 { a + b }\n".into(),
            ),
            (
                "tests/api.rs",
                "let app = Router::new().route(\"/x\", get(x));\n".into(),
            ),
        ]);
        let selection = cartographer_code(&files, &ContextBudget::CARTOGRAPHER);
        let order: Vec<_> = selection.excerpts.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            order,
            [
                "src/server.rs",
                "src/auth.rs",
                "tests/api.rs",
                "src/util.rs"
            ]
        );
        assert!(
            selection
                .render()
                .contains("    1| let app = Router::new()")
        );
    }

    #[test]
    fn budget_is_respected_and_left_out_is_reported() {
        let route = "app.get(\"/a\", handler);\n".repeat(50);
        let (_dir, files) = snapshot(&[
            ("a/server.js", route.clone()),
            ("b/server.js", route),
            ("c/plain.js", "const x = 1;\n".into()),
        ]);
        let budget = ContextBudget {
            code_bytes: 2_000,
            code_file_bytes: 1_500,
            ..ContextBudget::CARTOGRAPHER
        };
        let selection = cartographer_code(&files, &budget);
        assert!(selection.bytes <= 2_000);
        assert_eq!(selection.excerpts.len(), 2, "one big file + the small one");
        assert!(selection.excerpts[0].partial);
        assert_eq!(selection.left_out, ["b/server.js"]);
    }

    #[test]
    fn large_files_keep_header_and_windows_around_routes() {
        let mut lines: Vec<String> = (1..=400)
            .map(|n| format!("let filler_{n} = {n};"))
            .collect();
        lines[299] = "router.post(\"/pay\", pay);".into();
        let (_dir, files) = snapshot(&[("src/index.js", lines.join("\n"))]);
        let budget = ContextBudget {
            code_file_bytes: 4_000,
            ..ContextBudget::CARTOGRAPHER
        };
        let selection = cartographer_code(&files, &budget);
        let text = &selection.excerpts[0].text;
        assert!(
            text.starts_with("=== src/index.js (400 lines; excerpts) ==="),
            "{text}"
        );
        assert!(
            text.contains("  300| router.post(\"/pay\", pay);"),
            "{text}"
        );
        assert!(text.contains("    1| let filler_1"));
        assert!(!text.contains("  150| "));
    }

    #[test]
    fn config_prefers_deployment_files_and_never_secrets() {
        let (_dir, files) = snapshot(&[
            (".github/workflows/ci.yml", "on: push".into()),
            ("Cargo.toml", "[package]".into()),
            ("deploy/Dockerfile", "FROM rust".into()),
            (".env", "KEY=real".into()),
        ]);
        let selection = cartographer_config(&files, &ContextBudget::CARTOGRAPHER);
        let order: Vec<_> = selection.excerpts.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            order,
            [
                "deploy/Dockerfile",
                "Cargo.toml",
                ".github/workflows/ci.yml"
            ]
        );
        assert!(!selection.render().contains("KEY=real"));
    }
}
