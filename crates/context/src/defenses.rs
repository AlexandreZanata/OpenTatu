//! Supplement forward traces with upstream registration and its shared defenses.
//!
//! Explicit reverse imports discover a router that imports the handler, then the normal
//! forward selector discovers that router's middleware. This is bounded heuristic context,
//! never proof of reachability, coverage, or a wider incremental finding scope (R14).

use std::collections::BTreeSet;

use crate::related::{go_modules, import_targets};
use crate::selection::is_low_value;
use crate::{FileKind, Seed, Selection, SnapshotFiles, entry_point_code};

/// Maximum reverse import hops from the entry point's existing forward context.
const UPSTREAM_DEPTH: usize = 2;
/// Upstream files expanded with the normal forward selector, per hop.
const UPSTREAM_FRONTIER: usize = 12;

/// Entry-point code with relevant upstream registration and shared-defense context.
///
/// Seeds remain first. Forward and supplemental files are then interleaved so a large
/// dependency package cannot crowd out every upstream control. Every candidate excluded
/// by the byte budget is returned in `left_out`; partial excerpts retain their markers.
/// Every intermediate selection uses that same budget; reverse discovery retains paths
/// of omitted candidates without retaining their rendered bodies.
/// Empty seeds produce empty context. [`entry_point_code`] remains the incremental
/// dependency selector, so supplemental context does not expand the changed scope.
pub fn investigation_code(
    files: &SnapshotFiles,
    seeds: &[Seed],
    budget: usize,
    per_file: usize,
) -> Selection {
    let forward = entry_point_code(files, seeds, budget, per_file);
    let modules = go_modules(files);
    // Candidate paths survive packing, but discarded rendered bodies do not. Keeping
    // only paths for discovery bounds retained excerpt memory across the three passes.
    let mut known: BTreeSet<String> = forward
        .excerpts
        .iter()
        .map(|e| e.path.clone())
        .chain(forward.left_out.iter().cloned())
        .collect();
    if known.is_empty() {
        return forward;
    }
    let imports: Vec<(String, Vec<String>)> = files
        .files()
        .iter()
        .filter(|file| file.kind == FileKind::Source && !is_low_value(&file.path))
        .filter_map(|file| {
            let text = files.read_text(&file.path)?;
            let extension = file.path.rsplit_once('.').map_or("", |(_, ext)| ext);
            let targets = import_targets(&text.content, &file.path, extension, &modules);
            (!targets.is_empty()).then(|| (file.path.clone(), targets))
        })
        .collect();
    let mut supplemental = Vec::new();
    let mut omitted = forward.left_out.clone();
    let mut frontier = known.clone();
    for _ in 0..UPSTREAM_DEPTH {
        let upstream: Vec<String> = imports
            .iter()
            .filter(|(path, _)| !known.contains(path))
            .filter(|(_, targets)| {
                targets.iter().any(|target| {
                    frontier.iter().any(|path| {
                        target == path
                            || path.rsplit_once('/').is_some_and(|(dir, _)| dir == target)
                    })
                })
            })
            .map(|(path, _)| path.clone())
            .collect();
        frontier = upstream.iter().cloned().collect();
        if frontier.is_empty() {
            break;
        }
        // Even unexpanded upstream registrations remain visible as omissions (R14).
        omitted.extend(upstream.iter().skip(UPSTREAM_FRONTIER).cloned());
        let upstream_seeds: Vec<Seed> = upstream
            .iter()
            .take(UPSTREAM_FRONTIER)
            .map(|path| Seed {
                path: path.clone(),
                line: None,
            })
            .collect();
        let extra = entry_point_code(files, &upstream_seeds, budget, per_file);
        omitted.extend(extra.left_out.iter().cloned());
        for excerpt in extra.excerpts {
            if known.insert(excerpt.path.clone()) {
                supplemental.push(excerpt);
            }
        }
        known.extend(extra.left_out);
        // Do not rediscover upstream candidates that exceeded the expansion bound.
        known.extend(upstream);
    }
    let seed_paths: BTreeSet<&str> = seeds.iter().map(|seed| seed.path.as_str()).collect();
    let (seed_excerpts, related): (Vec<_>, Vec<_>) = forward
        .excerpts
        .into_iter()
        .partition(|excerpt| seed_paths.contains(excerpt.path.as_str()));
    let mut ordered = seed_excerpts;
    let mut related = related.into_iter();
    let mut supplemental = supplemental.into_iter();
    loop {
        let next = related.next();
        let defense = supplemental.next();
        if next.is_none() && defense.is_none() {
            break;
        }
        ordered.extend(next);
        ordered.extend(defense);
    }
    let mut selected = Selection::default();
    for excerpt in ordered {
        selected.push(excerpt, budget);
    }
    selected.left_out.extend(omitted);
    let included: BTreeSet<&str> = selected.excerpts.iter().map(|e| e.path.as_str()).collect();
    selected
        .left_out
        .retain(|path| !included.contains(path.as_str()));
    selected.left_out.sort();
    selected.left_out.dedup();
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(entries: &[(&str, &str)]) -> (tempfile::TempDir, SnapshotFiles) {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in entries {
            let file = dir.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, content).unwrap();
        }
        let files = SnapshotFiles::scan(dir.path()).unwrap();
        (dir, files)
    }

    fn seed(path: &str) -> Vec<Seed> {
        vec![Seed {
            path: path.into(),
            line: None,
        }]
    }

    fn paths(selection: &Selection) -> Vec<&str> {
        selection
            .excerpts
            .iter()
            .map(|excerpt| excerpt.path.as_str())
            .collect()
    }

    #[test]
    fn upstream_go_router_supplies_the_middleware_missing_from_forward_trace() {
        let (_dir, files) = snapshot(&[
            ("go.mod", "module example.com/app\n"),
            (
                "api/handler.go",
                "package api\nfunc GetOrder() { store.Get(id) }\n",
            ),
            (
                "cmd/server.go",
                "package main\nimport (\n\"example.com/app/api\"\n\"example.com/app/security\"\n)\nfunc main() { router.Use(security.RequireOwner); router.GET(\"/orders/:id\", api.GetOrder) }\n",
            ),
            (
                "security/owner.go",
                "package security\nfunc RequireOwner() { compareSessionOwner() }\n",
            ),
            (
                "other/start.go",
                "package other\nimport \"example.com/app/unrelated\"\n",
            ),
            (
                "unrelated/service.go",
                "package unrelated\nfunc Different() {}\n",
            ),
        ]);
        let seeds = seed("api/handler.go");
        assert_eq!(
            paths(&entry_point_code(&files, &seeds, 100_000, 20_000)),
            ["api/handler.go"]
        );
        let selected = investigation_code(&files, &seeds, 100_000, 20_000);
        assert_eq!(paths(&selected)[0], "api/handler.go");
        assert!(paths(&selected).contains(&"cmd/server.go"));
        assert!(selected.render().contains("compareSessionOwner"));
        assert!(!paths(&selected).contains(&"other/start.go"));
        assert!(!paths(&selected).contains(&"unrelated/service.go"));
        assert!(selected.left_out.is_empty());
    }

    #[test]
    fn upstream_typescript_import_keeps_defenses_without_expanding_incremental_scope() {
        let (_dir, files) = snapshot(&[
            (
                "routes/orders.ts",
                "export function getOrder(req) { return store.get(req.id); }\n",
            ),
            (
                "server.ts",
                "import { getOrder } from './routes/orders';\nimport { auth } from './security/auth';\napp.get('/orders/:id', auth, getOrder);\n",
            ),
            (
                "security/auth.ts",
                "export function auth(req) { return requireOwner(req); }\n",
            ),
            (
                "tests/server.ts",
                "import { getOrder } from '../routes/orders';\n",
            ),
            (".env", "SECRET=never-read-this\n"),
        ]);
        let seeds = seed("routes/orders.ts");
        let selected = investigation_code(&files, &seeds, 100_000, 20_000);
        assert!(paths(&selected).contains(&"server.ts"));
        assert!(paths(&selected).contains(&"security/auth.ts"));
        assert!(!paths(&selected).contains(&"tests/server.ts"));
        assert!(!selected.render().contains("never-read-this"));
        let forward = entry_point_code(&files, &seeds, 100_000, 20_000);
        assert!(!paths(&forward).contains(&"security/auth.ts"));
        assert!(
            investigation_code(&files, &[], 100_000, 20_000)
                .excerpts
                .is_empty()
        );
    }

    #[test]
    fn r14_context_is_deterministic_bounded_and_reports_supplemental_omissions() {
        let (_dir, files) = snapshot(&[
            ("routes/orders.ts", "export function getOrder() {}\n"),
            (
                "server.ts",
                "import { getOrder } from './routes/orders';\nimport { auth } from './security/auth';\n",
            ),
            ("security/auth.ts", "export function auth() { deny(); }\n"),
        ]);
        let seeds = seed("routes/orders.ts");
        let only_seed = entry_point_code(&files, &seeds, 100_000, 20_000);
        let selected = investigation_code(&files, &seeds, only_seed.bytes, 20_000);
        assert_eq!(paths(&selected), ["routes/orders.ts"]);
        assert_eq!(selected.left_out, ["security/auth.ts", "server.ts"]);
        assert!(selected.bytes <= only_seed.bytes);
        assert_eq!(
            selected,
            investigation_code(&files, &seeds, only_seed.bytes, 20_000)
        );
    }

    #[test]
    fn upstream_rust_and_python_registrations_reach_their_shared_controls() {
        let (_dir, files) = snapshot(&[
            ("src/api/orders.rs", "pub fn get_order() {}\n"),
            (
                "src/main.rs",
                "use crate::api::orders::get_order;\nuse crate::security::auth;\nfn main() { register(auth, get_order); }\n",
            ),
            ("src/security.rs", "pub fn auth() { require_owner(); }\n"),
            (
                "api/views.py",
                "def get_order(request):\n    return store.get(request.id)\n",
            ),
            (
                "api/routes.py",
                "from api.views import get_order\nfrom security.auth import owner\nregister(owner(get_order))\n",
            ),
            (
                "security/auth.py",
                "def owner(handler):\n    return enforce_owner(handler)\n",
            ),
        ]);
        let rust = investigation_code(&files, &seed("src/api/orders.rs"), 100_000, 20_000);
        assert!(paths(&rust).contains(&"src/main.rs"));
        assert!(paths(&rust).contains(&"src/security.rs"));
        let python = investigation_code(&files, &seed("api/views.py"), 100_000, 20_000);
        assert!(paths(&python).contains(&"api/routes.py"));
        assert!(paths(&python).contains(&"security/auth.py"));
    }

    #[test]
    fn r14_upstream_fanout_is_bounded_and_excess_registration_is_reported() {
        let (snapshot_dir, _) = snapshot(&[("api/orders.ts", "export function getOrder() {}\n")]);
        for index in 0..=UPSTREAM_FRONTIER {
            let dir = snapshot_dir.path().join(format!("server-{index:02}"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("main.ts"),
                "import { getOrder } from '../api/orders';\n",
            )
            .unwrap();
        }
        let files = SnapshotFiles::scan(snapshot_dir.path()).unwrap();
        let selected = investigation_code(&files, &seed("api/orders.ts"), 100_000, 20_000);
        assert_eq!(selected.excerpts.len(), 1 + UPSTREAM_FRONTIER);
        assert_eq!(selected.left_out, ["server-12/main.ts"]);
    }

    #[test]
    fn r14_budgeted_intermediates_keep_omitted_paths_for_upstream_discovery() {
        let large = "export function helper() { return store.get(id); }\n".repeat(500);
        let (_dir, files) = snapshot(&[
            (
                "entry/handler.ts",
                "import { helper } from '../service/helper';\nexport function GET() { return helper(); }\n",
            ),
            ("service/helper.ts", &large),
            (
                "server/main.ts",
                "import { helper } from '../service/helper';\nimport { auth } from '../security/auth';\nregister(auth, helper);\n",
            ),
            (
                "security/auth.ts",
                "export function auth() { requireOwner(); }\n",
            ),
        ]);
        let seeds = seed("entry/handler.ts");
        let forward = entry_point_code(&files, &seeds, 600, 5_000);
        assert_eq!(forward.left_out, ["service/helper.ts"]);
        let selected = investigation_code(&files, &seeds, 600, 5_000);
        assert!(selected.bytes <= 600);
        assert!(paths(&selected).contains(&"server/main.ts"));
        assert!(paths(&selected).contains(&"security/auth.ts"));
        assert_eq!(selected.left_out, ["service/helper.ts"]);
        let empty = investigation_code(&files, &seeds, 0, 5_000);
        assert!(empty.excerpts.is_empty());
        assert!(empty.left_out.contains(&"entry/handler.ts".to_owned()));
        assert!(empty.left_out.contains(&"server/main.ts".to_owned()));
    }

    #[test]
    fn r14_upstream_file_reached_through_an_expanded_import_is_not_reported_omitted() {
        let (snapshot_dir, _) = snapshot(&[("api/orders.ts", "export function getOrder() {}\n")]);
        for index in 0..=UPSTREAM_FRONTIER {
            let dir = snapshot_dir.path().join(format!("server-{index:02}"));
            std::fs::create_dir_all(&dir).unwrap();
            let mut content = "import { getOrder } from '../api/orders';\n".to_owned();
            if index == 0 {
                content.push_str("import { bridge } from '../server-12/main';\n");
            }
            std::fs::write(dir.join("main.ts"), content).unwrap();
        }
        let files = SnapshotFiles::scan(snapshot_dir.path()).unwrap();
        let selected = investigation_code(&files, &seed("api/orders.ts"), 100_000, 20_000);
        assert!(paths(&selected).contains(&"server-12/main.ts"));
        assert!(selected.left_out.is_empty());
    }
}
