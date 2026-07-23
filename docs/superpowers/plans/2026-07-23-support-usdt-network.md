# Fang Support USDT Network Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** State that Fang accepts USDT on BNB Smart Chain and Ethereum while removing the separate crypto-transfer warning.

**Architecture:** Keep the existing wallet data model and copy behavior. Change only the USDT subtitle, remove the warning markup/styles, and protect the copy with a source-level frontend regression test.

**Tech Stack:** Svelte 5, Node.js 22 test runner, Vite

## Global Constraints

- The exact subtitle is `BNB Smart Chain (BEP20) · Ethereum (ERC20)`.
- The existing USDT address and copy behavior remain unchanged.
- The responsible-donation guidance remains unchanged.
- The complete `.safety` warning markup and CSS are removed.

---

### Task 1: Update and verify the Support screen

**Files:**
- Create: `app/src/lib/support-content.test.js`
- Modify: `app/src/screens/Support.svelte`

**Interfaces:**
- Consumes: static `WALLETS` data and Support screen markup.
- Produces: the approved USDT network label without the removed warning.

- [ ] **Step 1: Write the failing source regression test**

```javascript
import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';

const source = fs.readFileSync(new URL('../screens/Support.svelte', import.meta.url), 'utf8');

test('USDT lists the accepted BNB and Ethereum networks', () => {
  assert.match(source, /BNB Smart Chain \(BEP20\) · Ethereum \(ERC20\)/);
  assert.doesNotMatch(source, /Confirm network before sending/);
});

test('the removed transfer warning and styles stay absent', () => {
  assert.doesNotMatch(source, /Crypto transfers cannot be reversed/);
  assert.doesNotMatch(source, /class="safety"/);
  assert.doesNotMatch(source, /\.safety(?:\s|:|\{)/);
});
```

- [ ] **Step 2: Run the focused test and confirm failure**

Run: `node --test app/src/lib/support-content.test.js`

Expected: FAIL because the approved network label is absent and warning remains.

- [ ] **Step 3: Apply the approved Svelte change**

Replace the USDT `network` field with `BNB Smart Chain (BEP20) · Ethereum (ERC20)`. Delete the complete `<div class="safety">…</div>` block and the `.safety`, `.safety :global(svg)`, and `.safety p` selectors while preserving `.copy-error`.

- [ ] **Step 4: Run frontend verification**

Run:

```bash
npm test --prefix app
npm run build --prefix app
```

Expected: all frontend tests pass and Vite builds successfully.

- [ ] **Step 5: Commit**

```bash
git add app/src/screens/Support.svelte app/src/lib/support-content.test.js
git commit -m "feat(support): list accepted USDT networks"
```
