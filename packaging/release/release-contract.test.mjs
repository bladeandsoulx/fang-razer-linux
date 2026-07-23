import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import {
  checksumNames,
  inspectDeb,
  releaseNames,
  stageRelease,
  validateManifest
} from './release-contract.mjs';

test('0.9.4 owns six exact assets and five checksum entries', () => {
  assert.deepEqual(releaseNames('0.9.4'), [
    'install.sh',
    'SHA256SUMS',
    'Fang_0.9.4_amd64.deb',
    'fangd_0.9.4-1_amd64.deb',
    'fang-0.9.4-1.x86_64.rpm',
    'fangd-0.9.4-1.x86_64.rpm'
  ]);
  assert.deepEqual(
    checksumNames('0.9.4'),
    releaseNames('0.9.4').filter((name) => name !== 'SHA256SUMS')
  );
});

test('manifest rejects missing, duplicate, malformed, path, and extra entries', () => {
  const expected = checksumNames('0.9.4');
  const valid = expected.map((name) => `${'a'.repeat(64)}  ${name}\n`).join('');
  assert.doesNotThrow(() => validateManifest(valid, expected));

  for (const malformed of [
    valid.replace(/^.*\n/, ''),
    valid + valid.split('\n')[0] + '\n',
    valid.replace(/[a-f0-9]{64}/, 'BAD'),
    valid.replace('install.sh', '../install.sh'),
    valid + `${'b'.repeat(64)}  seventh.asset\n`,
    valid.slice(0, -1)
  ]) {
    assert.throws(() => validateManifest(malformed, expected));
  }
});

test('DEB metadata inspection queries every field independently', () => {
  const calls = [];
  const values = new Map([
    ['Package', 'fang'],
    ['Version', '0.9.4'],
    ['Architecture', 'amd64']
  ]);

  const metadata = inspectDeb('Fang_0.9.4_amd64.deb', (command, args) => {
    calls.push([command, args]);
    return values.get(args.at(-1));
  });

  assert.deepEqual(metadata, { name: 'fang', version: '0.9.4', arch: 'amd64' });
  assert.deepEqual(calls, [
    ['dpkg-deb', ['-f', 'Fang_0.9.4_amd64.deb', 'Package']],
    ['dpkg-deb', ['-f', 'Fang_0.9.4_amd64.deb', 'Version']],
    ['dpkg-deb', ['-f', 'Fang_0.9.4_amd64.deb', 'Architecture']]
  ]);
});

test('stageRelease creates a deterministic six-asset directory', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'fang-release-contract-'));
  const debDir = path.join(root, 'deb');
  const rpmDir = path.join(root, 'rpm');
  const outputDir = path.join(root, 'out');
  fs.mkdirSync(debDir);
  fs.mkdirSync(rpmDir);
  const installer = path.join(root, 'install.sh');
  fs.writeFileSync(installer, '#!/usr/bin/env bash\n');

  for (const name of releaseNames('0.9.4').slice(2)) {
    const dir = name.endsWith('.deb') ? debDir : rpmDir;
    fs.writeFileSync(path.join(dir, name), name);
  }

  stageRelease({
    version: '0.9.4',
    debDir,
    rpmDir,
    outputDir,
    installer,
    inspectDeb(file) {
      return path.basename(file).startsWith('Fang_')
        ? { name: 'fang', version: '0.9.4', arch: 'amd64' }
        : { name: 'fangd', version: '0.9.4-1', arch: 'amd64' };
    },
    inspectRpm(file) {
      return path.basename(file).startsWith('fang-')
        ? { name: 'fang', epoch: '0', version: '0.9.4', release: '1', arch: 'x86_64' }
        : { name: 'fangd', epoch: '(none)', version: '0.9.4', release: '1', arch: 'x86_64' };
    }
  });

  assert.deepEqual(fs.readdirSync(outputDir).sort(), releaseNames('0.9.4').sort());
  const manifest = fs.readFileSync(path.join(outputDir, 'SHA256SUMS'), 'utf8');
  validateManifest(manifest, checksumNames('0.9.4'));
  assert.deepEqual(
    manifest.trimEnd().split('\n').map((line) => line.slice(66)),
    checksumNames('0.9.4')
  );
  fs.rmSync(root, { recursive: true });
});

test('stageRelease rejects package metadata mismatches before staging', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'fang-release-contract-bad-'));
  const debDir = path.join(root, 'deb');
  const rpmDir = path.join(root, 'rpm');
  const outputDir = path.join(root, 'out');
  fs.mkdirSync(debDir);
  fs.mkdirSync(rpmDir);
  const installer = path.join(root, 'install.sh');
  fs.writeFileSync(installer, '#!/usr/bin/env bash\n');
  for (const name of releaseNames('0.9.4').slice(2)) {
    fs.writeFileSync(path.join(name.endsWith('.deb') ? debDir : rpmDir, name), name);
  }

  assert.throws(
    () =>
      stageRelease({
        version: '0.9.4',
        debDir,
        rpmDir,
        outputDir,
        installer,
        inspectDeb: () => ({ name: 'wrong', version: '0.9.4', arch: 'amd64' }),
        inspectRpm: () => ({
          name: 'fang',
          epoch: '0',
          version: '0.9.4',
          release: '1',
          arch: 'x86_64'
        })
      }),
    /metadata/
  );
  assert.equal(fs.existsSync(outputDir), false);
  fs.rmSync(root, { recursive: true });
});

test('documentation exposes release, review, integrity, manual, and source install paths', () => {
  const repositoryRoot = path.resolve(path.dirname(new URL(import.meta.url).pathname), '../..');
  const readme = fs.readFileSync(path.join(repositoryRoot, 'README.md'), 'utf8');
  const contributing = fs.readFileSync(path.join(repositoryRoot, 'CONTRIBUTING.md'), 'utf8');
  const hardware = fs.readFileSync(path.join(repositoryRoot, 'HARDWARE_TESTING.md'), 'utf8');
  const sourceInstaller = path.join(repositoryRoot, 'packaging/install-from-source.sh');

  assert.equal(fs.existsSync(path.join(repositoryRoot, 'packaging/install.sh')), false);
  assert.ok(fs.statSync(sourceInstaller).mode & 0o111);
  assert.match(readme, /## Install in one command/);
  assert.match(
    readme,
    /1\. Open Terminal\..*2\. Paste the command below and press Enter\..*3\. Open Fang from your app menu/s
  );
  assert.match(
    readme,
    /curl -fsSL https:\/\/github\.com\/bladeandsoulx\/fang-razer-linux\/releases\/latest\/download\/install\.sh \| bash/
  );
  assert.match(readme, /curl -fLO .*releases\/latest\/download\/install\.sh/);
  assert.match(readme, /less install\.sh\nbash install\.sh/);
  assert.match(readme, /releases\/download\/v0\.9\.4\/\{install\.sh,SHA256SUMS\}/);
  assert.match(readme, /sha256sum --check .*install\.sh/);
  assert.match(readme, /Ubuntu 22\.04.*Ubuntu 24\.04.*Debian 12.*Debian 13/s);
  assert.match(readme, /Fedora 43.*Fedora 44/s);
  assert.match(readme, /without `sudo`|not with `sudo`/);
  assert.match(readme, /refuses downgrades/i);
  assert.match(readme, /manual package installation/i);
  assert.match(readme, /packaging\/install-from-source\.sh/);
  assert.match(contributing, /IMMUTABLE_RELEASES_TOKEN/);
  assert.match(contributing, /read-only.*Administration|Administration.*read-only/is);
  assert.match(hardware, /packaging\/install-from-source\.sh/);
});
