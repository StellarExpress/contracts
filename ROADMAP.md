# Roadmap

## Shipped

- [x] Core shipment lifecycle: create, accept, confirm pickup, confirm
      delivery
- [x] Two-milestone escrow release (`pickup_release_bps` split)
- [x] Sender-side cancel (while `Open`) and deadline-based reclaim
- [x] Dispute flow with a single configured arbiter
- [x] Marketplace read views (`list_open_shipments`,
      per-sender/per-carrier lookups)
- [x] `scripts/deploy.sh` for testnet/futurenet/local deployment

## Near-term

- [ ] Third-party security audit before any mainnet deployment (see
      [SECURITY.md](SECURITY.md) — testnet-only until this lands)
- [ ] Mainnet deployment and a published, verified contract id
- [ ] Event emission (`env.events().publish(...)`) for shipment state
      transitions, so the backend can index activity without polling
      `get_shipment`
- [ ] Partial/multi-item shipments (currently one `total_amount` per
      shipment id)

## Mid-term / under consideration

- [ ] Multiple arbiters or a dispute-committee model, instead of a
      single `arbiter` address configured at `init`
- [ ] Configurable per-category deadlines or default
      `pickup_release_bps` presets
- [ ] Contract upgradeability — deliberately out of scope today; the
      contract has no admin override beyond `init`, and any upgrade
      path needs its own audit before it's added
- [ ] Batch operations (e.g. bulk `list_open_shipments` filtering by
      category or asset)

Have a proposal? Open an issue — see [CONTRIBUTING.md](CONTRIBUTING.md).
