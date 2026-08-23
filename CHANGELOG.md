# Changelog

All notable changes to the `escrow` contract are documented here.
This project follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- Project scaffold: contributor guide, code of conduct, security
  policy, license, and supporting docs.

## [0.1.0]

### Added
- Initial `escrow` contract (`contracts/escrow`): a single deployed
  instance managing every shipment, identified by a `u64` id.
- `init(admin, arbiter)` one-time setup, guarded against
  re-initialization.
- Shipment lifecycle: `create_shipment` (funds escrow via the
  shipment's Stellar Asset Contract `token`), `accept_shipment`,
  `confirm_pickup` (pays out `pickup_release_bps`% immediately),
  `confirm_delivery` (pays the remaining balance), `cancel_shipment`
  (full refund while still `Open`), and `reclaim_expired` (refund
  after `delivery_deadline_ledger` with no carrier).
- Dispute flow: `raise_dispute` (callable by sender, carrier, or
  receiver) and `resolve_dispute` (arbiter-only, splits the remaining
  balance by `sender_bps`).
- Read-only views: `get_shipment`, `list_shipments_by_sender`,
  `list_shipments_by_carrier`, `list_open_shipments`.
- Structured `#[contracterror]` `Error` enum (14 variants) instead of
  panics, so callers get typed failure reasons.
- 14 unit tests (`contracts/escrow/src/test.rs`) covering the full
  happy path, wrong-carrier/wrong-receiver rejection, cancel/reclaim
  guards, dispute resolution, and invalid split validation.
- `scripts/deploy.sh` — builds and deploys to a given network
  (`local` | `testnet` | `futurenet` | `mainnet`), writing the
  resulting contract id to `.contract-id.<network>`.
