import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const panel = fs.readFileSync(path.join(root, 'app/src/screens/Changelog.svelte'), 'utf8');
const changelog = fs.readFileSync(path.join(root, 'CHANGELOG.md'), 'utf8');

test('the in-app changelog contains the latest releases in descending order', () => {
  const v094 = panel.indexOf("version: '0.9.4'");
  const v093 = panel.indexOf("version: '0.9.3'");
  const v092 = panel.indexOf("version: '0.9.2'");

  assert.ok(v094 >= 0, 'v0.9.4 must be present');
  assert.ok(v093 > v094, 'v0.9.3 must follow v0.9.4');
  assert.ok(v092 > v093, 'v0.9.2 must follow v0.9.3');
});

test('v0.9.4 records installer, USDT network, and warning-removal changes', () => {
  assert.match(panel, /release-locked one-command installer/i);
  assert.match(panel, /BNB Smart Chain \(BEP20\).*Ethereum \(ERC20\)/);
  assert.match(panel, /generic crypto-transfer warning.*were removed/i);
  assert.match(
    changelog,
    /## \[0\.9\.4\][\s\S]*?### Removed[\s\S]*?generic crypto-transfer warning/i
  );
});
