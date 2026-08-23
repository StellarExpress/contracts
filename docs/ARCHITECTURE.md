# Architecture

This describes the actual implementation in `contracts/escrow/src/`
(`lib.rs`, `types.rs`, `errors.rs`) — see [README.md](../README.md)
for a narrative walkthrough and the full method reference.

## Storage layout

The contract uses a single `DataKey` enum
(`contracts/escrow/src/types.rs`) across two storage categories:

- **Instance storage**: `DataKey::Admin`, `DataKey::Arbiter` — set
  once at `init` — and `DataKey::NextShipmentId`, an incrementing
  counter read on every `create_shipment` call.
- **Persistent storage**, keyed per shipment and per party:
  - `DataKey::Shipment(shipment_id)` -> the full `Shipment` record
  - `DataKey::SenderShipments(sender)` -> `Vec<u64>` of shipment ids
  - `DataKey::CarrierShipments(carrier)` -> `Vec<u64>` of shipment ids
  - `DataKey::OpenShipments` -> `Vec<u64>`, the marketplace feed of
    shipments waiting for a carrier

`OpenShipments` is maintained as an explicit list rather than derived
by scanning, so `list_open_shipments()` is a single storage read
regardless of how many shipments have ever been created.

## The `Shipment` state machine

`ShipmentStatus` (`types.rs`) moves in one direction, with two exits
from `Open` and one detour from `Accepted`/`InTransit`:

```
Open --accept_shipment--> Accepted --confirm_pickup--> InTransit --confirm_delivery--> Delivered
  |                          |                            |
  |--cancel_shipment-->  Cancelled                        |
  |--reclaim_expired-->  Expired                           |
                            \--raise_dispute-----------------/
                                        |
                                        v
                                   Disputed --resolve_dispute--> Resolved
```

Every transition is enforced with a `matches!(shipment.status, ...)`
guard in `lib.rs` before any state or funds move — e.g.
`confirm_pickup` rejects unless `status == Accepted`, and
`cancel_shipment` rejects unless `status == Open`. There is no way to
skip a state or move backwards.

## Party roles and authorization

Every state-changing method takes an explicit party `Address` and
calls `.require_auth()` on it — the contract never infers who is
allowed to act from `env.invoker()` or similar. There is no admin
override for any per-shipment action; `admin` (set at `init`) is not
read anywhere else in `lib.rs`.

| Role | Set by / when | Can call |
|---|---|---|
| **Sender** | Caller of `create_shipment` | `cancel_shipment` (while `Open`), `reclaim_expired` (after deadline, while `Open`), `raise_dispute` |
| **Carrier** | Whoever calls `accept_shipment` first on an `Open` shipment | `confirm_pickup`, `raise_dispute` |
| **Receiver** | Set by the sender in `create_shipment`; need not be a platform account | `confirm_delivery`, `raise_dispute` |
| **Arbiter** | Set once at `init(admin, arbiter)` | `resolve_dispute` only |

`require_carrier` (a private helper in `lib.rs`) distinguishes "no
carrier yet" (`Error::ShipmentNotOpen`) from "a different carrier
already claimed this" (`Error::WrongCarrier`) so callers get a
specific reason, not a generic auth failure.

## Fund movement

The contract never holds a private accounting ledger for value — every
transfer goes through the standard Stellar Asset Contract interface
via `token::Client`:

- `create_shipment` calls `token_client.transfer(sender, contract, total_amount)`
  to pull funds into escrow.
- `transfer_out` (a private helper) wraps
  `token_client.transfer(contract, to, amount)` for every payout path:
  the pickup share in `confirm_pickup`, the remaining balance in
  `confirm_delivery`, refunds in `cancel_shipment` /
  `reclaim_expired`, and both shares of a resolved dispute in
  `resolve_dispute`.
- `shipment.released_amount` is the running total already paid out; it
  is checked against `total_amount` rather than trusted from caller
  input, so a shipment can never release more than it was funded with.

## Basis points math

`pickup_release_bps` and `resolve_dispute`'s `sender_bps` are both
validated to be `<= 10_000` (100%) before use. Splits are computed as
`(amount * bps) / 10_000` using `i128` arithmetic — the same asset
precision Soroban token contracts use — and any dispute remainder goes
to whichever side gets the second calculation
(`remaining - sender_share` for the carrier), so the two shares always
sum exactly to `remaining` with no rounding leakage.

## Why ledger sequence, not timestamps

`delivery_deadline_ledger` (checked in `reclaim_expired` against
`env.ledger().sequence()`) uses the Stellar ledger's monotonic
sequence number rather than `env.ledger().timestamp()`. Both are
available on `Env`, but the sequence number is the value every
validator agrees on deterministically at close, which matters for a
condition that unlocks a sender's refund.
