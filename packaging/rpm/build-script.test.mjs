import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

test('RPM build script uses custom specs and disables Tauri bundling', () => {
  const script = fs.readFileSync(path.join(root, 'packaging/rpm/build.sh'), 'utf8');
  assert.match(script, /node app\/scripts\/version\.mjs check/);
  assert.match(script, /cargo build --release -p fangd/);
  assert.match(script, /npm run tauri build -- --no-bundle/);
  assert.match(script, /rpmbuild .*fangd\.spec/s);
  assert.match(script, /rpmbuild .*fang\.spec/s);
  assert.doesNotMatch(script, /--bundles rpm/);
});
