# <img src="assets/logo.svg" width="32" height="32" align="center" alt="" /> StellarExpress — Contracts

Soroban smart contracts powering **StellarExpress**, a logistics platform
built on Stellar. Ship food, packages, and general goods with a payment
escrow that no single party controls — a carrier gets paid on pickup and
delivery only when the on-chain conditions are actually met, not when
someone says so.

> Ship it. Track it. Trust the escrow, not the courier.

This repo contains a single contract, `escrow`, plus the tooling to
build, test, and deploy it. It's one of three StellarExpress repos:

| Repo | Purpose |
|---|---|
| [`contracts`](https://github.com/StellarExpress/contracts) *(this repo)* | The Soroban `escrow` contract |
| [`backend`](https://github.com/StellarExpress/backend) | GraphQL API, Postgres data layer, non-custodial Stellar integration |
| [`frontend`](https://github.com/StellarExpress/frontend) | Marketing site + product preview (Next.js) |

## Table of contents

- [New to Stellar/Soroban? Start here](#new-to-stellarsoroban-start-here)
- [Why an escrow contract, not just a payment on delivery](#why-an-escrow-contract-not-just-a-payment-on-delivery)
- [Core model](#core-model)
- [Method reference](#method-reference)
- [Errors](#errors)
- [Storage layout](#storage-layout)
- [A walkthrough: a food delivery from Lagos to Abuja](#a-walkthrough-a-food-delivery-from-lagos-to-abuja)
- [Development](#development)
- [Testing](#testing)
- [Deployment](#deployment)
- [Design principles](#design-principles)
- [Contributing](#contributing)

## New to Stellar/Soroban? Start here

- **Stellar** is a public payments network — a shared ledger anyone can
  read, and with the right permission, write to. Transactions confirm in
  3–5 seconds for a fraction of a cent, which matters for a logistics
  platform moving lots of small, time-sensitive payments.
- **Soroban** is Stellar's smart contract platform — code that lives on
  the ledger and runs exactly as written, which nobody (including
  StellarExpress) can quietly change after the fact. That's the entire
  reason the escrow lives here instead of in a database: a payment rule
  enforced by a smart contract is a rule that actually holds even if the
  carrier and the platform disagree about what should happen.
- **An escrow** just means funds are held by a neutral third party until
  agreed conditions are met, instead of being paid directly to the
  carrier upfront (risking the goods never show up) or only after
  delivery (leaving the carrier to front the cost of transport with no
  guarantee of payment). This contract *is* that neutral third party —
  code, not a company.
- **Signing / authorization.** Nothing moves without a signature from the
  relevant private key — `caller.require_auth()` in this contract means
  "this call only proceeds if `caller` cryptographically proved they
  authorized it in this transaction." Nobody can forge that, including
  StellarExpress.
- **A Stellar Asset Contract (SAC)** is how a currency — XLM, USDC, or a
  custom token — looks to a Soroban contract: another contract with a
  standard `transfer`/`balance` interface. This contract never invents
  its own accounting; it always calls into the real asset's contract.
- **The ledger** is Stellar's version of a block — a sequence number
  that increases roughly every 5 seconds. This contract uses ledger
  sequence numbers, not wall-clock time, to track delivery deadlines,
  since that's the one clock every node on the network agrees on.
- **Basis points (bps)** are percentages with more precision — 100 bps =
  1%, so 10,000 bps = 100%. `pickup_release_bps` and dispute-resolution
  splits are expressed this way so a 33.5% split, say, is representable
  exactly.

## Why an escrow contract, not just a payment on delivery

A logistics deal has three parties with misaligned incentives at every
step: the sender wants proof the goods will actually move before paying
in full, the carrier wants some guarantee of payment before spending
fuel and time, and the receiver wants to withhold final payment until
the goods are actually in hand. A single "pay on delivery" transfer
protects only the sender; a single "pay upfront" transfer protects only
the carrier. **Splitting the payment into two on-chain milestones** —
part on confirmed pickup, the rest on confirmed delivery — protects
everyone without requiring any of them to trust each other, only the
contract.

## Core model

`contracts/escrow` is a **single deployed contract instance** that
manages every shipment on the platform, each identified by a `u64` id
(the same multi-tenant-per-contract pattern used across StellarExpress's
sibling projects, rather than deploying a fresh contract per shipment).
It holds **no custodial keys of its own beyond the escrowed funds** —
every state-changing call requires the relevant party's own
`require_auth()`, and every payout happens through the standard Stellar
Asset Contract `token` interface.

| Concept | What it is |
|---|---|
| **Shipment** | One delivery job. Denominated in a single Stellar asset (XLM, USDC, or any SAC). Tracks `total_amount`, `released_amount`, `status`, and the `pickup_release_bps` split. |
| **Sender** | Funds the escrow when creating the shipment. Can cancel for a full refund only while the shipment is still `Open` (no carrier yet), or reclaim funds if the deadline passes with no carrier. |
| **Carrier** | Claims an `Open` shipment (`accept_shipment`), then confirms pickup and is paid `pickup_release_bps`% immediately — compensation for the trip regardless of what happens next. |
| **Receiver** | Confirms delivery, releasing the remaining balance to the carrier. Set by the sender at creation time; doesn't need to be a platform account. |
| **Arbiter** | Configured once at contract `init`. The only address that can resolve a dispute, splitting whatever remains in escrow between sender and carrier by a percentage the arbiter decides. |
| **Milestone split** | `pickup_release_bps` (e.g. `5000` = 50%) is set per shipment by the sender at creation — a fragile or high-trust shipment might release less upfront; a bulky or low-risk one might release more. |

Status moves in one direction: `Open → Accepted → InTransit → Delivered`,
with `Cancelled` / `Expired` as exits from `Open`, and `Disputed →
Resolved` as a detour from `Accepted`/`InTransit`.

## Method reference

All methods live on `EscrowContract` in
`contracts/escrow/src/lib.rs`. Every state-changing call takes an
explicit party address and calls `.require_auth()` on it.

**Setup**

| Method | Auth | Description |
|---|---|---|
| `init(admin, arbiter)` | `admin` | One-time setup; fails if already initialized. |
| `get_arbiter()` | none | Read-only. |

**Shipment lifecycle**

| Method | Auth | Description |
|---|---|---|
| `create_shipment(sender, receiver, asset, total_amount, pickup_release_bps, delivery_deadline_ledger, category)` | `sender` | Transfers `total_amount` into escrow immediately; shipment starts `Open`. Returns the new `shipment_id`. |
| `accept_shipment(shipment_id, carrier)` | `carrier` | Claims an `Open` shipment; moves it to `Accepted`. |
| `confirm_pickup(shipment_id, carrier)` | the assigned `carrier` | Pays out `pickup_release_bps`% immediately; moves to `InTransit`. |
| `confirm_delivery(shipment_id, receiver)` | the shipment's `receiver` | Pays the remaining balance to the carrier; moves to `Delivered`. |
| `cancel_shipment(shipment_id, sender)` | `sender` | Full refund — only while still `Open`. |
| `reclaim_expired(shipment_id, sender)` | `sender` | Full refund if still `Open` past `delivery_deadline_ledger`. |

**Disputes**

| Method | Auth | Description |
|---|---|---|
| `raise_dispute(shipment_id, caller)` | sender, carrier, or receiver | Freezes an `Accepted`/`InTransit` shipment pending arbiter review. |
| `resolve_dispute(shipment_id, arbiter, sender_bps)` | the configured `arbiter` | Splits whatever remains between sender (`sender_bps`%) and carrier (the rest). |

**Read-only views**

| Method | Description |
|---|---|
| `get_shipment(shipment_id)` | Full shipment record. |
| `list_shipments_by_sender(sender)` | Shipment ids a sender created. |
| `list_shipments_by_carrier(carrier)` | Shipment ids a carrier has accepted. |
| `list_open_shipments()` | Ids of shipments waiting for a carrier — the marketplace feed. |

## Errors

`Error` (`contracts/escrow/src/errors.rs`) is a `#[contracterror]` enum,
so callers get a structured code, not a panic string:
`AlreadyInitialized`, `NotInitialized`, `NotAuthorized`,
`ShipmentNotFound`, `InvalidAmount`, `InvalidReleaseSplit`,
`ShipmentNotOpen`, `ShipmentAlreadyAccepted`, `WrongCarrier`,
`WrongReceiver`, `InvalidStatusForAction`, `NotExpiredYet`,
`InvalidDisputeSplit`, `NotDisputed`.

## Storage layout

Everything lives in **persistent** storage, keyed by a `DataKey` enum
(`contracts/escrow/src/types.rs`):

```
DataKey::Admin / DataKey::Arbiter          -> Address (instance storage — set once at init)
DataKey::NextShipmentId                    -> u64 (instance storage)
DataKey::Shipment(shipment_id)              -> Shipment
DataKey::SenderShipments(sender)            -> Vec<u64>
DataKey::CarrierShipments(carrier)          -> Vec<u64>
DataKey::OpenShipments                      -> Vec<u64>  (the marketplace feed)
```

## A walkthrough: a food delivery from Lagos to Abuja

```text
1. init(admin=Platform, arbiter=TrustedArbiter)   // once, at deployment

2. create_shipment(sender=Amaka, receiver=Chidi, asset=USDC_SAC,
                    total_amount=5000, pickup_release_bps=4000,
                    delivery_deadline_ledger=..., category=Food)
   -> shipment_id = 1, status=Open, 5000 USDC now held in escrow

3. accept_shipment(1, carrier=DejiLogistics)
   -> status=Accepted

4. confirm_pickup(1, DejiLogistics)
   -> 40% (2000 USDC) paid to Deji immediately, status=InTransit
   -> Deji is compensated for the trip even before Chidi confirms anything

5. confirm_delivery(1, receiver=Chidi)
   -> remaining 60% (3000 USDC) paid to Deji, status=Delivered

   -- if something had gone wrong instead --
   raise_dispute(1, caller=Amaka)          // status=Disputed
   resolve_dispute(1, arbiter=TrustedArbiter, sender_bps=7000)
   -> of whatever remained in escrow, Amaka gets 70% back, Deji gets 30%
```

## Development

Requires the [Stellar CLI](https://developers.stellar.org/docs/tools/cli)
and the `wasm32v1-none` Rust target. Soroban contracts compile to
**WebAssembly**, a small sandboxed binary format, so a public network
can run anyone's contract without it touching the host machine directly:

```bash
rustup target add wasm32v1-none
```

```bash
# run the full test suite (unit tests, in-memory Soroban Env)
cargo test --workspace

# format & lint
cargo fmt --all
cargo build --workspace          # native build, fastest feedback loop

# build the deployable wasm artifact
stellar contract build
# equivalent to: cargo build --target wasm32v1-none --release -p escrow
```

## Testing

`contracts/escrow/src/test.rs` has **14 tests** covering: double-init
rejection, escrow funding on shipment creation, carrier acceptance
(including rejecting a double-accept), the pickup-share payout, the
full pickup-then-delivery payout, wrong-carrier and wrong-receiver
rejection, cancellation while still open (and rejection once accepted),
reclaiming after the deadline (and rejection before it), the full
dispute → resolution payout split, non-arbiter dispute rejection, and
invalid release-split validation.

## Deployment

```bash
./scripts/deploy.sh testnet <your-source-account>
```

Builds, optimizes, and deploys in one step, writing the contract id to
`.contract-id.<network>` (gitignored). **Call `init(admin, arbiter)`
once immediately after deploying** — the script prints the exact
`stellar contract invoke` command to do it. Pass `mainnet`, `futurenet`,
or `local` as the first argument for other networks.

## Design principles

- **Non-custodial.** The contract holds escrowed funds, but no single
  party — not StellarExpress, not the arbiter, not the carrier — can
  move them outside the rules the contract enforces.
- **Milestone payment aligns incentives.** Splitting payment at pickup
  and delivery means the carrier isn't fronting the entire job on trust,
  and the sender isn't paying in full for goods that haven't moved yet.
- **The arbiter is a last resort, not a gatekeeper.** Every shipment
  resolves without the arbiter in the normal case (pickup → delivery
  confirmation); the arbiter only ever touches a shipment that's been
  explicitly disputed by a party to it.
- **Permissionless where trust isn't needed.** Anyone can call
  `accept_shipment` on an open listing or `list_open_shipments` to browse
  the marketplace — the contract's own checks are the security boundary,
  not caller identity.

## Contributing

Issues and PRs are welcome. Before opening a PR: run `cargo fmt --all`,
`cargo test --workspace`, and make sure `cargo build --target
wasm32v1-none --release --workspace` still succeeds. See
[`StellarExpress/backend`](https://github.com/StellarExpress/backend) for
how the API builds unsigned XDR against this contract, and
[`StellarExpress/frontend`](https://github.com/StellarExpress/frontend)
for the product surface built on top of it.
