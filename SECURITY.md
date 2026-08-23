# Security Policy

## Reporting a vulnerability

`escrow` holds real funds in transit between senders, carriers, and
receivers — a bug here isn't just a bad API response, it's a way for
someone to move money they shouldn't be able to touch. If you find a
security issue (an authorization bypass on any `require_auth()` check,
a way to release or withhold funds outside the pickup/delivery split
the sender configured, an integer overflow in the bps math, a way for
the arbiter's `resolve_dispute` split to be manipulated by a
non-arbiter, a way to double-spend or re-enter a shipment, etc.),
**please do not open a public issue.**

Instead, report it privately by contacting the maintainers through the
StellarExpress GitHub organization (see [MAINTAINERS.md](MAINTAINERS.md)).
Include:

- The affected method(s) in `contracts/escrow/src/lib.rs`
- A minimal repro — ideally a failing test against the harness in
  `contracts/escrow/src/test.rs`
- The impact: which party's funds are at risk and how

We'll acknowledge reports as quickly as we can and aim to have a fix
(or a mitigation plan) before any public disclosure.

## Scope

This contract has not had a third-party audit. Treat any deployment as
testnet-only until one has been completed — see
[ROADMAP.md](ROADMAP.md). Findings in test code, tooling
(`scripts/`), or documentation are welcome but are not fund-affecting
and can be filed as normal public issues.

## Supported versions

| Version | Supported |
|---|---|
| 0.1.x | :white_check_mark: (testnet only, pre-audit) |
