.PHONY: build test fmt fmt-check lint wasm deploy clean

# Native build — fastest feedback loop, mirrors `cargo build --workspace`
# from the README.
build:
	cargo build --workspace

# Full unit test suite (in-memory Soroban Env, no network required).
test:
	cargo test --workspace

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets -- -D warnings

# Deployable wasm artifact, same as `stellar contract build`
# (contracts/escrow/Cargo.toml -> target/wasm32v1-none/release/escrow.wasm).
wasm:
	stellar contract build

# Builds + deploys via scripts/deploy.sh. Defaults to testnet; override
# with `make deploy NETWORK=futurenet SOURCE=my-account`.
NETWORK ?= testnet
SOURCE ?= escrow-deployer
deploy:
	./scripts/deploy.sh $(NETWORK) $(SOURCE)

clean:
	cargo clean
