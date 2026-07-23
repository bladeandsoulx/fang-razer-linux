import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const fixtureVersion = JSON.parse(fs.readFileSync(path.join(root, 'app/package.json'), 'utf8')).version;
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
  fs.writeFileSync(
    path.join(dir, 'install.sh'),
    `#!/usr/bin/env bash\nreadonly VERSION='${fixtureVersion}'\nreadonly RELEASE_TAG='v${fixtureVersion}'\n`
  );
  return dir;
}

function run(dir, ...args) {
  return spawnSync(process.execPath, ['app/scripts/version.mjs', ...args], {
    cwd: dir,
    encoding: 'utf8'
  });
}

function mutateFixture(text, pattern, replacement) {
  const mutated = text.replace(pattern, replacement);
  assert.notEqual(mutated, text, 'fixture mutation must change the source text');
  return mutated;
}

test('check rejects an incorrect RPM upper bound', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fang.spec');
  const malformed = mutateFixture(
    fs.readFileSync(spec, 'utf8'),
    /^(%global[^\S\r\n]+fangd_upper[^\S\r\n]+)\S+([^\S\r\n]*)$/m,
    (_match, prefix, suffix) => prefix + '999.999.999' + suffix
  );
  fs.writeFileSync(spec, malformed);
  const result = run(dir, 'check');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /RPM.*release line|fangd_upper/i);
  fs.rmSync(dir, { recursive: true });
});

test('check rejects a multiline RPM Version field', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fangd.spec');
  const malformed = mutateFixture(
    fs.readFileSync(spec, 'utf8'),
    /^(Version:)[^\S\r\n]*(\S+)[^\S\r\n]*$/m,
    '$1\n$2'
  );
  fs.writeFileSync(spec, malformed);
  const result = run(dir, 'check');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /could not read RPM Version/);
  fs.rmSync(dir, { recursive: true });
});

test('check rejects a multiline RPM macro value', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fang.spec');
  const malformed = mutateFixture(
    fs.readFileSync(spec, 'utf8'),
    /^(%global[^\S\r\n]+fangd_upper)[^\S\r\n]+(\S+)[^\S\r\n]*$/m,
    '$1\n$2'
  );
  fs.writeFileSync(spec, malformed);
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
  const installer = fs.readFileSync(path.join(dir, 'install.sh'), 'utf8');
  assert.match(installer, /^readonly VERSION='0\.10\.0'$/m);
  assert.match(installer, /^readonly RELEASE_TAG='v0\.10\.0'$/m);
  fs.rmSync(dir, { recursive: true });
});

test('check rejects stale release-installer identity', () => {
  const dir = fixture();
  const installer = path.join(dir, 'install.sh');
  fs.writeFileSync(
    installer,
    fs.readFileSync(installer, 'utf8').replace(`VERSION='${fixtureVersion}'`, "VERSION='9.8.7'")
  );
  const result = run(dir, 'check');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /release installer|synchronized/i);
  fs.rmSync(dir, { recursive: true });
});

test('set rejects a multiline RPM Version field', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fangd.spec');
  const malformed = mutateFixture(
    fs.readFileSync(spec, 'utf8'),
    /^(Version:)[^\S\r\n]*(\S+)[^\S\r\n]*$/m,
    '$1\n$2'
  );
  fs.writeFileSync(spec, malformed);
  const result = run(dir, 'set', '9.8.7');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /could not update RPM Version/);
  assert.equal(fs.readFileSync(spec, 'utf8'), malformed);
  fs.rmSync(dir, { recursive: true });
});

test('set rejects a multiline RPM macro value', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fang.spec');
  const malformed = mutateFixture(
    fs.readFileSync(spec, 'utf8'),
    /^(%global[^\S\r\n]+fangd_upper)[^\S\r\n]+(\S+)[^\S\r\n]*$/m,
    '$1\n$2'
  );
  fs.writeFileSync(spec, malformed);
  const result = run(dir, 'set', '9.8.7');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /could not update RPM macro fangd_upper/);
  assert.equal(fs.readFileSync(spec, 'utf8'), malformed);
  fs.rmSync(dir, { recursive: true });
});
