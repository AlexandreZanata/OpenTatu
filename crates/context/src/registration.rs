//! Focused upstream registration and the shared controls it actually references.
//!
//! Import membership alone is not evidence that a file registers a handler. Qualified
//! references to the seeded declarations identify windows; only referenced security
//! definitions from direct imports/same package supplement those windows (R14).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::related::{definition_lines, go_modules, import_targets};
use crate::selection::{is_low_value, is_security_identifier, render_focused_file};
use crate::{FileKind, Seed, Selection, SnapshotFiles};

const DEPTH: usize = 2;
pub(crate) const FRONTIER: usize = 12;
pub(crate) const WINDOW_BYTES: usize = 8_000;
const FOCUS_EDGE: usize = 16;

fn names(content: &str) -> BTreeSet<String> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            [
                "type ",
                "func ",
                "pub fn ",
                "pub async fn ",
                "export function ",
                "export async function ",
                "export class ",
                "export const ",
                "def ",
                "async def ",
            ]
            .iter()
            .find_map(|prefix| line.strip_prefix(prefix))
            .and_then(|rest| {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    Some(name)
                } else if rest.starts_with('(') {
                    // A Go receiver type can be declared in a sibling source file.
                    rest.split_once(')')
                        .and_then(|(receiver, _)| receiver.split_whitespace().last())
                        .map(|receiver| receiver.trim_start_matches('*').to_owned())
                } else {
                    None
                }
            })
        })
        .collect()
}

fn targets_path(target: &str, path: &str) -> bool {
    target == path || path.rsplit_once('/').is_some_and(|(dir, _)| dir == target)
}

fn identifiers(content: &str) -> BTreeSet<String> {
    content
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|word| word.len() >= 3)
        .map(str::to_owned)
        .collect()
}

fn reference_lines(
    path: &str,
    content: &str,
    frontier: &BTreeMap<String, BTreeSet<String>>,
    modules: &[(String, String)],
) -> (usize, Vec<usize>) {
    let extension = path.rsplit_once('.').map_or("", |(_, ext)| ext);
    let targets = import_targets(content, path, extension, modules);
    let related: Vec<_> = frontier
        .iter()
        .filter(|(seed, _)| targets.iter().any(|target| targets_path(target, seed)))
        .collect();
    if related.is_empty() {
        return (0, Vec::new());
    }
    let mut references = BTreeSet::new();
    if extension == "go" {
        for line in content.lines() {
            let imports = import_targets(line, path, extension, modules);
            let Some(start) = line.find('"') else {
                continue;
            };
            let spec = line[start + 1..].split('"').next().unwrap_or("");
            let prefix = line[..start]
                .trim()
                .strip_prefix("import ")
                .unwrap_or(line[..start].trim());
            let alias = if prefix.is_empty() {
                spec.rsplit('/').next().unwrap_or("")
            } else {
                prefix
            };
            for (seed, symbols) in &related {
                if imports.iter().any(|target| targets_path(target, seed)) {
                    references.extend(symbols.iter().map(|name| format!("{alias}.{name}")));
                }
            }
        }
    } else {
        references.extend(
            related
                .iter()
                .flat_map(|(_, symbols)| symbols.iter().cloned()),
        );
    }
    let matches = content
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with('#'))
                && references.iter().any(|name| {
                    line.match_indices(name).any(|(index, _)| {
                        let identifier = |c: char| c.is_alphanumeric() || c == '_';
                        line[..index].chars().last().is_none_or(|c| !identifier(c))
                            && line[index + name.len()..]
                                .chars()
                                .next()
                                .is_none_or(|c| !identifier(c))
                    })
                })
        })
        .map(|(index, _)| index);
    let mut head = Vec::new();
    let mut tail = VecDeque::new();
    let mut hits = 0usize;
    for index in matches {
        hits += 1;
        if head.len() < FOCUS_EDGE {
            head.push(index);
        } else {
            if tail.len() == FOCUS_EDGE {
                tail.pop_front();
            }
            tail.push_back(index);
        }
    }
    head.extend(tail);
    (hits, head)
}

/// Compact upstream and actual referenced controls, with bounded intermediate bodies.
pub(crate) fn registration_code(
    files: &SnapshotFiles,
    seeds: &[Seed],
    omitted_forward: &[String],
    budget: usize,
    per_file: usize,
) -> Selection {
    let modules = go_modules(files);
    let mut frontier: BTreeMap<String, BTreeSet<String>> = seeds
        .iter()
        .filter_map(|seed| {
            let text = files.read_text(&seed.path)?;
            Some((seed.path.clone(), names(&text.content)))
        })
        .collect();
    let mut visited: BTreeSet<String> = frontier.keys().cloned().collect();
    let mut registrations = Selection::default();
    let mut registration_focus: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut controls: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let per_file = per_file.min(WINDOW_BYTES);
    for depth in 0..DEPTH {
        let mut candidates = Vec::new();
        for fallback in [false, true] {
            if fallback {
                if !candidates.is_empty() || depth != 0 {
                    break;
                }
                // A handler with no importing caller can delegate to an omitted sink.
                // Use those paths only as a fallback, never the whole forward closure.
                frontier.extend(omitted_forward.iter().filter_map(|path| {
                    let text = files.read_text(path)?;
                    Some((path.clone(), names(&text.content)))
                }));
            }
            for file in files
                .files()
                .iter()
                .filter(|file| file.kind == FileKind::Source && !is_low_value(&file.path))
            {
                if visited.contains(&file.path) {
                    continue;
                }
                let Some(text) = files.read_text(&file.path) else {
                    continue;
                };
                let (hits, focus) = reference_lines(&file.path, &text.content, &frontier, &modules);
                if hits > 0 {
                    candidates.push((file.path.clone(), hits, focus));
                    candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                    if candidates.len() > FRONTIER
                        && let Some((path, _, _)) = candidates.pop()
                    {
                        visited.insert(path.clone());
                        registrations.left_out.push(path);
                    }
                }
            }
        }
        let mut next = BTreeMap::new();
        for (path, _, focus) in candidates {
            visited.insert(path.clone());
            let Some(text) = files.read_text(&path) else {
                continue;
            };
            next.insert(path.clone(), names(&text.content));
            let excerpt = render_focused_file(&path, &text.content, per_file, &focus);
            let referenced: BTreeSet<String> = identifiers(&excerpt.text)
                .into_iter()
                .filter(|name| is_security_identifier(name))
                .collect();
            let extension = path.rsplit_once('.').map_or("", |(_, ext)| ext);
            let targets = import_targets(&text.content, &path, extension, &modules);
            let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
            for file in files
                .files()
                .iter()
                .filter(|file| file.kind == FileKind::Source && !is_low_value(&file.path))
            {
                let same_package = file
                    .path
                    .rsplit_once('/')
                    .is_some_and(|(parent, _)| parent == dir);
                if file.path == path
                    || !(same_package
                        || targets
                            .iter()
                            .any(|target| targets_path(target, &file.path)))
                {
                    continue;
                }
                let Some(source) = files.read_text(&file.path) else {
                    continue;
                };
                let focus = definition_lines(&source.content, &referenced);
                if !focus.is_empty() {
                    controls.entry(file.path.clone()).or_default().extend(focus);
                }
            }
            registration_focus.insert(path, focus);
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }
    let mut result = Selection::default();
    for (path, mut focus) in controls {
        let Some(text) = files.read_text(&path) else {
            continue;
        };
        // A middleware list often passes function values without invoking them. Follow
        // those referenced control names within the same file, without an import sweep.
        for _ in 0..DEPTH {
            let excerpt = render_focused_file(&path, &text.content, per_file, &focus);
            let referenced = identifiers(&excerpt.text)
                .into_iter()
                .filter(|name| is_security_identifier(name))
                .collect();
            let more = definition_lines(&text.content, &referenced);
            for line in more {
                if !focus.contains(&line) {
                    focus.push(line);
                }
            }
        }
        if let Some(mut registration) = registration_focus.remove(&path) {
            // One source file may supply a registration and a shared control. Merge
            // focus before rendering: path deduplication must never hide the binding.
            for line in focus {
                if !registration.contains(&line) {
                    registration.push(line);
                }
            }
            focus = registration;
        }
        result.push(
            render_focused_file(&path, &text.content, per_file, &focus),
            budget,
        );
    }
    for (path, focus) in registration_focus {
        let Some(text) = files.read_text(&path) else {
            continue;
        };
        result.push(
            render_focused_file(&path, &text.content, per_file, &focus),
            budget,
        );
    }
    result.left_out.extend(registrations.left_out);
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

    #[test]
    fn r14_extreme_reference_density_retains_bounded_first_and_late_focus() {
        let mut content = "import { handler } from './handler';\n".to_owned();
        content.push_str(&"register(handler);\n".repeat(20_000));
        let final_line = content.lines().count();
        content.push_str("register(authMiddleware, handler);\n");
        let frontier = BTreeMap::from([("handler.ts".into(), BTreeSet::from(["handler".into()]))]);
        let (hits, focus) = reference_lines("router.ts", &content, &frontier, &[]);
        assert_eq!(hits, 20_002);
        assert_eq!(focus.len(), 2 * FOCUS_EDGE);
        assert_eq!(focus[0], 0);
        assert_eq!(focus.last(), Some(&final_line));
        let excerpt = render_focused_file("router.ts", &content, WINDOW_BYTES, &focus);
        assert!(excerpt.text.contains("register(authMiddleware, handler)"));
        assert!(excerpt.text.len() <= WINDOW_BYTES);
    }
}
