# OpenTatu specification

Normative English specification of OpenTatu. It is the translation of the owner's master
document (v1.0, pt-BR) with every name, format and identifier in English (ADR-0001).

| Part | File | Contents |
|---|---|---|
| A | [A-business-rules.md](A-business-rules.md) | Purpose, actors, entities, audit kinds, finding lifecycle, **rules R01–R20** |
| B | [B-memory-and-history.md](B-memory-and-history.md) | Event log, identifiers, dates, directory layout, record formats, fingerprint, MEMORY.md |
| C | [C-investigator.md](C-investigator.md) | AI roles, orchestrator flow, severity matrix, evidence standard |
| D | [D-llm-provider.md](D-llm-provider.md) | OpenCode Go integration: mandatory research and client rules |
| E | [E-platform.md](E-platform.md) | Platform principles, stack, tables, scanner isolation, MCP tools, self-security |

## How to cite

Cite by part and section, e.g. "spec §A.7 R04" or "spec §B.2". Section numbers match the
original document, so references stay stable.

## Precedence

1. **ADRs in [../DECISIONS.md](../DECISIONS.md) override this specification** where they
   say so (for example: local-first single binary instead of a dedicated VPS).
2. Ambiguities are listed in [../OPEN-QUESTIONS.md](../OPEN-QUESTIONS.md); do not resolve
   them on your own.
3. The AI role prompts (spec Part C) live in [../../prompts/](../../prompts/AGENTS.md) and are
   normative for the text sent to models.
4. The original "builder prompt" (Part F) is superseded by [../../AGENTS.md](../../AGENTS.md)
   and [../WORKFLOW.md](../WORKFLOW.md).

## Original document

The original pt-BR master document is archived in [original/](original/README.md). It is
**not normative** and its Portuguese names are invalid in code, formats and docs. Read it
only to resolve a doubt about the translation — and record that doubt in
[../OPEN-QUESTIONS.md](../OPEN-QUESTIONS.md).
