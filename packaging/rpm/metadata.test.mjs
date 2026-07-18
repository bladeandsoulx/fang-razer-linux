import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const read = (name) => fs.readFileSync(path.join(root, name), 'utf8');
const version = JSON.parse(read('app/package.json')).version;
const [major, minor] = version.split('.').map(Number);
const upper = `${major}.${minor + 1}.0`;

test('daemon spec uses native sysusers and a real license payload', () => {
  const spec = read('packaging/rpm/fangd.spec');
  assert.match(spec, new RegExp(`^Version:\\s*${version.replaceAll('.', '\\.')}\\s*$`, 'm'));
  assert.match(spec, /^Release:\s*1\s*$/m);
  assert.match(spec, /^License:\s*GPL-2\.0-only\s*$/m);
  assert.match(spec, /%\{_sysusersdir\}\/fang\.conf/);
  assert.match(spec, /%license %\{_licensedir\}\/%\{name\}\/LICENSE/);
  assert.match(spec, /%systemd_post fangd\.service/);
  assert.doesNotMatch(spec, /%sysusers_create_compat|groupadd|^%pre\s*$/m);
});

test('desktop spec owns strict daemon bounds and the tray runtime', () => {
  const spec = read('packaging/rpm/fang.spec');
  assert.match(spec, new RegExp(`^Version:\\s*${version.replaceAll('.', '\\.')}\\s*$`, 'm'));
  assert.match(spec, new RegExp(`^%global fangd_upper ${upper.replaceAll('.', '\\.')}\\s*$`, 'm'));
  assert.match(spec, /^Requires:\s*fangd >= %\{version\}\s*$/m);
  assert.match(spec, /^Requires:\s*fangd < %\{fangd_upper\}\s*$/m);
  assert.match(spec, /^Requires:\s*libayatana-appindicator-gtk3\s*$/m);
  assert.match(spec, /%license %\{_licensedir\}\/%\{name\}\/LICENSE/);
  assert.doesNotMatch(spec, /AutoReqProv:\s*no/);
});

test('sysusers and desktop files expose the required identities', () => {
  assert.equal(read('packaging/rpm/fang.sysusers'), 'g fang - -\n');
  const desktop = read('packaging/rpm/fang.desktop');
  assert.match(desktop, /^\[Desktop Entry\]$/m);
  assert.match(desktop, /^Exec=fang$/m);
  assert.match(desktop, /^Icon=fang$/m);
  assert.match(desktop, /^Terminal=false$/m);
});
