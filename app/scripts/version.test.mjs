import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const files = [
  'Cargo.toml',
  'Cargo.lock',
  'CHANGELOG.md',
  'app/package.json',
  'app/package-lock.json',
  'app/scripts/version.mjs',
  'app/src-tauri/Cargo.toml',
  'app/src-tauri/Cargo.lock',
  'app/src-tauri/tauri.conf.json',
  'packaging/rpm/fang.spec',
  'packaging/rpm/fangd.spec'
];

function fixture() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'fang-version-'));
  for (const name of files) {
    const destination = path.join(dir, name);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(path.join(root, name), destination);
  }
  return dir;
}

function run(dir, ...args) {
  return spawnSync(process.execPath, ['app/scripts/version.mjs', ...args], {
    cwd: dir,
    encoding: 'utf8'
  });
}

test('check rejects an incorrect RPM upper bound', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fang.spec');
  fs.writeFileSync(spec, fs.readFileSync(spec, 'utf8').replace('fangd_upper 0.10.0', 'fangd_upper 0.11.0'));
  const result = run(dir, 'check');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /RPM.*release line|fangd_upper/i);
  fs.rmSync(dir, { recursive: true });
});

test('check rejects a multiline RPM Version field', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fangd.spec');
  fs.writeFileSync(spec, fs.readFileSync(spec, 'utf8').replace('Version: 0.9.2', 'Version:\n0.9.2'));
  const result = run(dir, 'check');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /could not read RPM Version/);
  fs.rmSync(dir, { recursive: true });
});

test('check rejects a multiline RPM macro value', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fang.spec');
  fs.writeFileSync(
    spec,
    fs.readFileSync(spec, 'utf8').replace('%global fangd_upper 0.10.0', '%global fangd_upper\n0.10.0')
  );
  const result = run(dir, 'check');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /could not read RPM macro fangd_upper/);
  fs.rmSync(dir, { recursive: true });
});

test('set updates both RPM versions and the next-minor upper bound', () => {
  const dir = fixture();
  const result = run(dir, 'set', '0.10.0');
  assert.equal(result.status, 0, result.stdout + result.stderr);
  for (const name of ['packaging/rpm/fang.spec', 'packaging/rpm/fangd.spec']) {
    assert.match(fs.readFileSync(path.join(dir, name), 'utf8'), /^Version:[^\S\r\n]*0\.10\.0[^\S\r\n]*$/m);
  }
  assert.match(
    fs.readFileSync(path.join(dir, 'packaging/rpm/fang.spec'), 'utf8'),
    /^%global fangd_upper 0\.11\.0$/m
  );
  fs.rmSync(dir, { recursive: true });
});

test('set rejects a multiline RPM Version field', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fangd.spec');
  fs.writeFileSync(spec, fs.readFileSync(spec, 'utf8').replace('Version: 0.9.2', 'Version:\n0.9.2'));
  const result = run(dir, 'set', '0.9.3');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /could not update RPM Version/);
  assert.match(fs.readFileSync(spec, 'utf8'), /^Version:\n0\.9\.2$/m);
  fs.rmSync(dir, { recursive: true });
});

test('set rejects a multiline RPM macro value', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fang.spec');
  fs.writeFileSync(
    spec,
    fs.readFileSync(spec, 'utf8').replace('%global fangd_upper 0.10.0', '%global fangd_upper\n0.10.0')
  );
  const result = run(dir, 'set', '0.9.3');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /could not update RPM macro fangd_upper/);
  assert.match(fs.readFileSync(spec, 'utf8'), /^%global fangd_upper\n0\.10\.0$/m);
  fs.rmSync(dir, { recursive: true });
});
