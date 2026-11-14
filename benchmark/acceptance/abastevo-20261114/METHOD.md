# Context visibility method

This is a deterministic comparison of OpenTatu's context selectors, separate from any
model-driven vulnerability benchmark. It uses [Abastevo by Alexandre Zanata](https://github.com/AlexandreZanata/Abastevo)
at exact commit [`018f2df18c8b029ec3e665fc98ba7318e6f076bc`](https://github.com/AlexandreZanata/Abastevo/commit/018f2df18c8b029ec3e665fc98ba7318e6f076bc).
Abastevo's [Visible Attribution License 1.0](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/LICENSE)
and original creator attribution apply to that source. The published metrics contain
paths, line references and hashes, without target source bodies or credentials.

The full exact-commit archive was read from the existing local Git mirror and safely
extracted into scratch storage: 7,413 regular files, zero links or special files. Archive
paths were checked for absolute paths and parent traversal; no target executable,
configuration, build script, test, service or network request was run. OpenTatu's normal
safe index retained 7,407 files; the same index was passed to both selectors. This avoids
the incomplete backend/infra/docs extraction used for the separate manual source review.

Four real Go handler cases were selected manually: staff administrative catalog,
organization suspension, public contributor listing and map assertions. Each case has
predeclared source-line anchors for the handler, route registration, local controls,
upstream binding or shared middleware. The public-list case also identifies unrelated
administrative middleware: its visibility never establishes protection of that route.
The anchor list is a small diagnostic sample, not ground truth for vulnerabilities.

For each case, the helper calls `entry_point_code` and `investigation_code` with identical
seeds and two identical budgets: verifier 160,000 bytes or hunter 300,000 bytes, with
40,000 bytes per file. An anchor is visible only when its exact, numbered source line
occurs in the matching file excerpt. Merely selecting a file does not count. Both APIs
must respect the total byte budget, and an included candidate must not also be omitted.
The JSON records rendered bytes/hashes, selected files, partial files, omission counts,
the first 30 omitted paths, all anchor outcomes and gains/losses. Source-file hashes
and context-engine source hashes identify exactly what was measured.

The [initial calibration](context-comparison-initial.json) matches context source at
OpenTatu commit `49fa475`: it found no newly visible selected anchor in eight case/budget
pairs and lost the organization-suspension hunter's middleware-definition line. Merely
adding the composition-root file left its relevant binding lines outside excerpts, while
unrelated imported modules consumed the budget. That unfavorable result is preserved.

The [final comparison](context-comparison.json) measures focused registration windows,
directly referenced shared controls and preservation of the nearest forward trace. Both
runs use the same pinned target and the same predeclared anchors and budgets. All eight
baseline render hashes match the initial calibration. Repeating the final helper run
produced byte-identical JSON, and every recorded engine-source hash matched the source.

Six of eight case/budget pairs gain relevant source lines, with no selected-anchor loss:

- Staff catalog: both budgets newly show the administrative enablement condition,
  middleware-group binding and handler-group binding at `backend/cmd/api/main.go:928`,
  `:952` and `:954`. Existing middleware-definition and session-check anchors remain.
- Organization suspension: both budgets newly show the enablement guard and binding in
  `backend/internal/platform/enterprisesuite/admin.go:16` and `:28`. The verifier also
  gains the shared middleware-definition anchor. The hunter retains that existing line.
- Map assertions: both budgets newly show the exact composition-root registration at
  `backend/cmd/api/main.go:301` while retaining the local cursor-validation anchor.
- Public contributor listing: selected anchors are unchanged, including its local CORS
  binding and origin check. Shared administrative middleware remains unrelated context.

These are thirteen newly visible selected line checks across the eight measurements;
they establish a concrete context-visibility improvement on this diagnostic sample. They
do not measure model quality, vulnerability discovery, exploitability, recall or false
positives. Other forward excerpts can still be displaced; organization-suspension runs
retain explicit omissions (99 verifier candidates, 84 hunter candidates).

## Reproduce

Use a fresh archive of the pinned source, preserving repository-relative paths. Compile
only OpenTatu and the supplied helper; never build or run the Abastevo target. From the
OpenTatu repository, after the normal workspace build has generated dependency libraries:

```bash
python3 - /path/to/pinned-abastevo-snapshot <<'PY'
import pathlib, subprocess, sys, tempfile

root = pathlib.Path.cwd()
snapshot = pathlib.Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix='opentatu-context-probe-') as temporary:
    scratch = pathlib.Path(temporary)
    source = scratch / 'main.rs'
    source.write_bytes((root / 'benchmark/acceptance/abastevo-20261114/context-probe.rs.txt').read_bytes())
    binary = scratch / 'context-probe'
    libraries = root / 'target/debug/deps'
    command = ['rustc', '--edition=2024', str(source), '-L', f'dependency={libraries}', '-o', str(binary)]
    for name in ['opentatu_context', 'serde_json', 'sha2']:
        library = max(libraries.glob('lib' + name + '-*.rlib'), key=lambda path: path.stat().st_mtime)
        command.extend(['--extern', f'{name}={library}'])
    subprocess.run(command, check=True)
    subprocess.run([str(binary), str(snapshot), str(root)], check=True)
PY
```

The helper emits JSON to stdout. Rebuild current OpenTatu libraries before reproducing;
retained compiled libraries from a different source version are not comparable. The
metric includes hashes of the current context source files and the rendered selections.

## Limits

- The selectors use language/import heuristics, bounded traversal and excerpts. This
  comparison establishes visibility of specific lines, not complete data-flow coverage.
- The eight measurements share one source snapshot and four manually chosen cases;
  the same cases informed the refinement and are not a held-out validation set. Results
  cannot be generalized to the entire application or other repositories.
- More supplemental context can displace forward context within the same budget.
  Missing lines and partial files remain limitations, even without a new hypothesis.
- Upstream traversal starts from handler/type references and is limited to two hops and
  twelve retained candidate focus vectors per hop. Each file retains the first sixteen
  and last sixteen reference indices. Registration windows precede shared-control windows
  when both belong to one file. This is a bounded heuristic, not semantic reachability.
- Supplemental windows use at most 8K bytes per file and a quarter of the total budget
  with an 8K floor clipped to that total. Direct imports/same-package definitions must
  match referenced security identifiers. This avoids sweeping every imported module.
- Import discovery is rebuilt per call; this probe does not claim runtime or memory
  performance improvement. Topology caching is deferred until profiling supports it.
- No model was called, no API cost was incurred, and no target test or vulnerability
  reproduction was executed. There is no vulnerability recall denominator.
