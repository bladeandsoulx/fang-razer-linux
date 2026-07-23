# Fang Support screen USDT network label

**Date:** 2026-07-23  
**Status:** Approved

## Goal

Make the Tether donation card state exactly which networks Fang accepts,
without retaining the separate general crypto-transfer warning.

## Design

- Keep the existing USDT wallet address and copy behavior unchanged.
- Replace the USDT network subtitle with:
  `BNB Smart Chain (BEP20) · Ethereum (ERC20)`.
- Remove the complete crypto-transfer warning block below the wallet cards.
- Remove the warning block's now-unused `.safety` styles.
- Keep the responsible-donation guidance and copy-error handling unchanged.

## Verification

- A source-level frontend test asserts that both accepted USDT networks and
  token standards appear in the Support screen.
- The same test asserts that the removed warning text and `.safety` block no
  longer appear.
- The frontend test suite and production build must pass.
