# Contributing to smartdrop-contracts

Thanks for considering a contribution to the StellarExpress `escrow`
contract. This repo holds the on-chain half of the platform; see
[README.md](README.md) for the full contract model (shipments,
milestone releases, disputes) before diving into the code.

## Development setup

You'll need the [Stellar CLI](https://developers.stellar.org/docs/tools/cli)
and the `wasm32v1-none` Rust target:

```bash
rustup target add wasm32v1-none
```

`rust-toolchain.toml` pins the toolchain channel, so `rustup` will pick
up the right one automatically once you're in the repo.

```bash
# run the full test suite (unit tests, in-memory Soroban Env)
cargo test --workspace

# native build — fastest feedback loop
cargo build --workspace

# build the deployable wasm artifact
stellar contract build
```

`make build`, `make test`, and `make wasm` wrap the same commands — see
the [Makefile](Makefile).

## Before opening a PR

- `cargo fmt --all` (checked in CI as `cargo fmt --all -- --check`)
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo build --target wasm32v1-none --release --workspace` to confirm
  the wasm artifact still builds

All of the above run in `.github/workflows/ci.yml` on every push and
pull request to `main`; a red CI check will block merge.

If you touched `contracts/escrow/src/lib.rs`, add or update a test in
`contracts/escrow/src/test.rs` covering the change — every
state-changing method needs at least a happy-path test and, where it
matters for fund safety, the relevant auth/status-guard rejection
test.

## Branching and commits

- Branch off `main`; name branches by what they do
  (`fix/dispute-split-rounding`, `feat/batch-shipment-lookup`).
- Keep commits scoped to one logical change. Prefer imperative subject
  lines ("Add dispute split rounding test", not "Added" or "Adding").
- Reference the relevant issue number in the PR description.

## Reporting issues

Open a GitHub issue with a minimal reproduction. For contract bugs —
especially anything touching fund movement (`transfer_out`, the
pickup/delivery split, dispute resolution) — a failing test case
against `contracts/escrow/src/test.rs` is the most useful thing you
can attach. If the bug could let someone move funds they shouldn't be
able to, see [SECURITY.md](SECURITY.md) instead of filing a public
issue.
