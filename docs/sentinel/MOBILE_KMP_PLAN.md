# Superseded OpenTatu mobile plan

Status: superseded by the owner's 2026-11-14 decision and ADR-0038.

OpenTatu will not have a separate Android, KMP or iOS application. It remains a Rust
service managed through the existing ZaturoBot bot and applications. The local OpenTatu
dashboard and CLI remain operational support interfaces of the same binary.

The canonical replacement is [ZaturoBot integration](ZATUROBOT_INTEGRATION_PLAN.md).
S3 plans a minimal alert handoff; S7 plans authenticated service management and projections.
Client presentation, platform storage and distribution belong to ZaturoBot in a later,
separately authorized cross-repository task. The former mobile requirements for privacy,
freshness, replay, synchronization and exact approvals remain integration acceptance
requirements; a notification or ordinary conversation cannot authorize an action.

This file preserves historical links only. It no longer specifies a `mobile/` source
tree, Gradle build, OpenTatu-branded application, platform push implementation or release.
