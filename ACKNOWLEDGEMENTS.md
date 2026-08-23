# Acknowledgements

- Built on the [Soroban SDK](https://github.com/stellar/rs-soroban-sdk)
  and the [Stellar CLI](https://github.com/stellar/stellar-cli), and
  deployed to the [Stellar](https://stellar.org) network.
- Funds move through the standard Stellar Asset Contract (SAC)
  `transfer`/`balance` interface rather than any custom accounting, so
  this contract is compatible with any SEP-41-style token already
  deployed on Stellar (XLM, USDC, or a custom asset).
- The two-milestone escrow (pay a share on pickup, the rest on
  delivery) follows the same general pattern used by conditional
  payment/escrow designs across the wider Stellar and Ethereum smart
  contract ecosystems.
- Thanks to everyone in [CONTRIBUTORS.md](CONTRIBUTORS.md) for code,
  reviews, and issue reports.
