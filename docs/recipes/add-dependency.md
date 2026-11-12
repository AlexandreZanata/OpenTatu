# Recipe: add an external dependency

Every dependency costs build time, binary size and supply-chain risk. The bar is high.

1. Check whether the standard library or an existing dependency already does it
   (`Cargo.toml` `[workspace.dependencies]`).
2. Prefer crates that are widely used, maintained, with few transitive dependencies and an
   MSRV compatible with `rust-toolchain.toml`. Check the latest version with
   `cargo search <crate>` / `cargo info <crate>`.
3. Declare it **once** in the root `Cargo.toml` `[workspace.dependencies]` with
   `default-features = false` and only the features you need.
4. In the crate: `<name> = { workspace = true }` (add `features = [...]` only if this crate
   needs more). Any other form fails `cargo xtask arch`.
5. Heavy or security-relevant additions (HTTP stacks, crypto, parsers of untrusted input,
   anything with `build.rs` or native code) need an ADR.
6. Run `cargo xtask check` and commit `Cargo.lock` together.
