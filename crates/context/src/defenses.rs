//! Supplement forward traces with upstream registration and its shared defenses.
//!
//! Qualified reverse-import references discover the handler's upstream registration and
//! its directly referenced controls. This is bounded heuristic context,
//! never proof of reachability, coverage, or a wider incremental finding scope (R14).

use std::collections::{BTreeMap, BTreeSet};

use crate::registration::{WINDOW_BYTES, registration_code};
use crate::{Seed, Selection, SnapshotFiles, entry_point_code};

/// Entry-point traces supplemented by focused upstream registration and shared controls.
///
/// The existing forward selector owns dependency scope. Seeds and the two nearest
/// forward files remain first; referenced forward controls are promoted intact before
/// supplemental windows. Remaining forward excerpts retain their original order. Only a
/// quarter of the total budget (with an 8K floor, clipped to the total) is available for
/// supplemental candidates, and every
/// intermediate selection is bounded. Omitted paths are explicit and never also included.
pub fn investigation_code(
    files: &SnapshotFiles,
    seeds: &[Seed],
    budget: usize,
    per_file: usize,
) -> Selection {
    let forward = entry_point_code(files, seeds, budget, per_file);
    if seeds.is_empty() {
        return forward;
    }
    let supplemental = registration_code(
        files,
        seeds,
        &forward.left_out,
        budget.min((budget / 4).max(WINDOW_BYTES)),
        per_file,
    );
    let seed_paths: BTreeSet<&str> = seeds.iter().map(|seed| seed.path.as_str()).collect();
    let mut protected: BTreeSet<String> =
        seed_paths.iter().map(|path| (*path).to_owned()).collect();
    protected.extend(
        forward
            .excerpts
            .iter()
            .filter(|excerpt| !seed_paths.contains(excerpt.path.as_str()))
            .take(2)
            .map(|excerpt| excerpt.path.clone()),
    );
    // Preserve original rank separately from the lookup used for promoting shared controls.
    let order = forward
        .excerpts
        .iter()
        .map(|excerpt| excerpt.path.clone())
        .collect::<Vec<_>>();
    let mut original: BTreeMap<String, _> = forward
        .excerpts
        .into_iter()
        .map(|excerpt| (excerpt.path.clone(), excerpt))
        .collect();
    let mut result = Selection::default();
    for path in &order {
        if protected.contains(path)
            && let Some(excerpt) = original.remove(path)
        {
            result.push(excerpt, budget);
        }
    }
    for excerpt in supplemental.excerpts {
        if protected.contains(&excerpt.path) {
            continue;
        }
        let excerpt = original.remove(&excerpt.path).unwrap_or(excerpt);
        protected.insert(excerpt.path.clone());
        result.push(excerpt, budget);
    }
    for path in order {
        if let Some(excerpt) = original.remove(&path) {
            result.push(excerpt, budget);
        }
    }
    result.left_out.extend(forward.left_out);
    result.left_out.extend(supplemental.left_out);
    let included: BTreeSet<&str> = result
        .excerpts
        .iter()
        .map(|excerpt| excerpt.path.as_str())
        .collect();
    result
        .left_out
        .retain(|path| !included.contains(path.as_str()));
    result.left_out.sort();
    result.left_out.dedup();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registration::FRONTIER as UPSTREAM_FRONTIER;

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
            let mut content =
                "import { getOrder } from '../api/orders';\nexport function authBridge() {}\n"
                    .to_owned();
            if index == 0 {
                content.push_str("import { authBridge } from '../server-12/main';\n");
            }
            std::fs::write(dir.join("main.ts"), content).unwrap();
        }
        let files = SnapshotFiles::scan(snapshot_dir.path()).unwrap();
        let selected = investigation_code(&files, &seed("api/orders.ts"), 100_000, 20_000);
        assert!(paths(&selected).contains(&"server-12/main.ts"));
        assert!(selected.left_out.is_empty());
    }

    #[test]
    fn late_upstream_binding_is_visible_without_losing_forward_sink_or_shared_control() {
        let mut router = "package main\nimport (\n handlers \"example.com/app/handlers\"\n auth \"example.com/app/auth\"\n _ \"example.com/app/unrelated\"\n)\n".to_owned();
        router.push_str(&"// unrelated router setup\n".repeat(1_000));
        router.push_str("func main() {\n if enabled {\n r.Use(auth.OwnerMiddleware)\n h := handlers.Handler{}\n h.RegisterRoutes(r)\n }\n}\n");
        let (_dir, files) = snapshot(&[
            ("go.mod", "module example.com/app\n"),
            (
                "handlers/http.go",
                "package handlers\nimport (\n \"example.com/app/auth\"\n \"example.com/app/store\"\n)\ntype Handler struct{}\nfunc (h Handler) RegisterRoutes(r Router) { r.Get(\"/records\", h.read) }\nfunc (h Handler) read() { auth.CheckOwner(); store.ReadRecord() }\n",
            ),
            (
                "auth/owner.go",
                "package auth\nfunc OwnerMiddleware() { CheckOwner() }\nfunc CheckOwner() { compareSessionOwner() }\n",
            ),
            (
                "store/records.go",
                "package store\nfunc ReadRecord() { db.Query(\"SELECT owner_id FROM records\") }\n",
            ),
            ("cmd/main.go", &router),
            (
                "unrelated/helpers.go",
                "package unrelated\nfunc LargeHelper() { neverReviewUnreferencedDependency() }\n",
            ),
        ]);
        let seeds = seed("handlers/http.go");
        let before = entry_point_code(&files, &seeds, 12_000, 8_000);
        let after = investigation_code(&files, &seeds, 12_000, 8_000);
        assert!(before.render().contains("compareSessionOwner"));
        assert!(
            after.render().contains("compareSessionOwner"),
            "shared control must survive"
        );
        assert!(
            after.render().contains("SELECT owner_id FROM records"),
            "forward sink must survive"
        );
        assert!(
            after.render().contains("r.Use(auth.OwnerMiddleware)"),
            "late upstream group binding must be shown"
        );
        assert!(after.render().contains("if enabled"));
        assert!(!after.render().contains("neverReviewUnreferencedDependency"));
        assert!(after.bytes <= 12_000);
    }

    #[test]
    fn shared_control_and_registration_in_the_same_large_file_keep_both_windows() {
        let padding = "// unrelated setup padding padding padding\n".repeat(450);
        let router = format!(
            "import {{ getOrder }} from './routes/orders';\n{padding}export function authBridge(req) {{ return requireOwner(req); }}\n{padding}app.get('/orders/:id', authBridge, getOrder);\n"
        );
        let (_dir, files) = snapshot(&[
            (
                "routes/orders.ts",
                "export function getOrder(req) { return store.get(req.id); }\n",
            ),
            ("server.ts", &router),
            (
                "bootstrap.ts",
                "import { authBridge } from './server';\napp.use(authBridge);\n",
            ),
        ]);
        let selected = investigation_code(&files, &seed("routes/orders.ts"), 100_000, 20_000);
        let rendered = selected.render();
        assert!(rendered.contains("app.get('/orders/:id', authBridge, getOrder)"));
        assert!(rendered.contains("return requireOwner(req)"));
        assert_eq!(
            paths(&selected)
                .iter()
                .filter(|path| **path == "server.ts")
                .count(),
            1
        );
        assert!(selected.left_out.is_empty());
    }
}
