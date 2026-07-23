import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';

const source = fs.readFileSync(new URL('../screens/Support.svelte', import.meta.url), 'utf8');
const readme = fs.readFileSync(new URL('../../../README.md', import.meta.url), 'utf8');

test('USDT lists the accepted BNB and Ethereum networks', () => {
  assert.match(source, /BNB Smart Chain \(BEP20\) · Ethereum \(ERC20\)/);
  assert.doesNotMatch(source, /Confirm network before sending/);
});

test('the removed transfer warning and styles stay absent', () => {
  assert.doesNotMatch(source, /Crypto transfers cannot be reversed/);
  assert.doesNotMatch(source, /class="safety"/);
  assert.doesNotMatch(source, /\.safety(?:\s|:|\{)/);
});

test('README names both accepted USDT networks', () => {
  assert.match(
    readme,
    /USDT[\s\S]*?BNB Smart Chain \(BEP20\)[\s\S]*?Ethereum \(ERC20\)/
  );
});
