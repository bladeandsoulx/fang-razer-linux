#!/usr/bin/env node

// Keep every independently packaged Fang component on one release version.
// Usage:
//   node app/scripts/version.mjs check
//   node app/scripts/version.mjs set 0.8.1

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const file = (name) => path.join(root, name);
const read = (name) => fs.readFileSync(file(name), 'utf8');
const write = (name, value) => fs.writeFileSync(file(name), value);

function capture(label, text, pattern) {
  const match = text.match(pattern);
  if (!match) throw new Error('could not read ' + label);
  return match[1];
}

function cargoPackageVersion(text, name) {
  const escaped = name.replace(/[.*+?^{}$()|[\]\\]/g, '\\$&');
  return capture(
    name + ' lockfile version',
    text,
    new RegExp('\\[\\[package\\]\\]\\nname = "' + escaped + '"\\nversion = "([^"]+)"')
  );
}

function replaceRequired(name, text, pattern, replacement) {
  if (!pattern.test(text)) throw new Error('could not update ' + name);
  pattern.lastIndex = 0;
  return text.replace(pattern, replacement);
}

function replaceCargoPackageVersion(text, name, version) {
  const escaped = name.replace(/[.*+?^{}$()|[\]\\]/g, '\\$&');
  return replaceRequired(
    name + ' lockfile version',
    text,
    new RegExp('(\\[\\[package\\]\\]\\nname = "' + escaped + '"\\nversion = ")[^"]+(")'),
    '$1' + version + '$2'
  );
}

function rpmField(text, field) {
  return capture(
    'RPM ' + field,
    text,
    new RegExp('^' + field + ':[^\\S\\r\\n]*(\\S+)[^\\S\\r\\n]*$', 'm')
  );
}

function rpmMacro(text, name) {
  return capture(
    'RPM macro ' + name,
    text,
    new RegExp('^%global[^\\S\\r\\n]+' + name + '[^\\S\\r\\n]+(\\S+)[^\\S\\r\\n]*$', 'm')
  );
}

function replaceRpmField(text, field, value) {
  return replaceRequired(
    'RPM ' + field,
    text,
    new RegExp('^(' + field + ':[^\\S\\r\\n]*)\\S+([^\\S\\r\\n]*)$', 'm'),
    '$1' + value + '$2'
  );
}

function replaceRpmMacro(text, name, value) {
  return replaceRequired(
    'RPM macro ' + name,
    text,
    new RegExp('^(%global[^\\S\\r\\n]+' + name + '[^\\S\\r\\n]+)\\S+([^\\S\\r\\n]*)$', 'm'),
    '$1' + value + '$2'
  );
}

function currentVersions() {
  const rootCargo = read('Cargo.toml');
  const rootLock = read('Cargo.lock');
  const appCargo = read('app/src-tauri/Cargo.toml');
  const appLock = read('app/src-tauri/Cargo.lock');
  const packageJson = JSON.parse(read('app/package.json'));
  const packageLock = JSON.parse(read('app/package-lock.json'));
  const tauri = JSON.parse(read('app/src-tauri/tauri.conf.json'));
  const changelog = read('CHANGELOG.md');
  const installer = read('install.sh');
  const fangRpm = read('packaging/rpm/fang.spec');
  const fangdRpm = read('packaging/rpm/fangd.spec');
  return [
    ['workspace Cargo.toml', capture('workspace version', rootCargo, /\[workspace\.package\][\s\S]*?\nversion = "([^"]+)"/)],
    ['fang-protocol Cargo.lock', cargoPackageVersion(rootLock, 'fang-protocol')],
    ['fangd Cargo.lock', cargoPackageVersion(rootLock, 'fangd')],
    ['app package.json', packageJson.version],
    ['app package-lock.json', packageLock.version],
    ['app package-lock root', packageLock.packages[''].version],
    ['Tauri Cargo.toml', capture('Tauri package version', appCargo, /\[package\][\s\S]*?\nversion = "([^"]+)"/)],
    ['fang Tauri Cargo.lock', cargoPackageVersion(appLock, 'fang')],
    ['fang-protocol Tauri Cargo.lock', cargoPackageVersion(appLock, 'fang-protocol')],
    ['tauri.conf.json', tauri.version],
    ['fang RPM spec', rpmField(fangRpm, 'Version')],
    ['fangd RPM spec', rpmField(fangdRpm, 'Version')],
    ['release installer', capture('release installer version', installer, /^readonly VERSION='([^']+)'$/m)],
    ['CHANGELOG.md', capture('latest changelog release', changelog, /^## \[(\d+\.\d+\.\d+)\]/m)]
  ];
}

function check() {
  const versions = currentVersions();
  const expected = versions[0][1];
  const mismatches = versions.filter(([, version]) => version !== expected);
  if (mismatches.length) {
    for (const [name, version] of versions) {
      console.error(name + ': ' + version);
    }
    throw new Error('Fang release versions are not synchronized');
  }

  const tauri = JSON.parse(read('app/src-tauri/tauri.conf.json'));
  const depends = tauri.bundle.linux.deb.depends ?? [];
  const [major, minor] = expected.split('.').map(Number);
  const upper = major + '.' + (minor + 1) + '.0';
  const fangRpm = read('packaging/rpm/fang.spec');
  const installer = read('install.sh');
  if (
    capture('release installer tag', installer, /^readonly RELEASE_TAG='([^']+)'$/m) !==
    'v' + expected
  ) {
    throw new Error('release installer tag is not synchronized');
  }
  if (
    !/^Requires:\s*fangd >= %\{version\}\s*$/m.test(fangRpm) ||
    !/^Requires:\s*fangd < %\{fangd_upper\}\s*$/m.test(fangRpm) ||
    rpmMacro(fangRpm, 'fangd_upper') !== upper
  ) {
    throw new Error('Fang RPM must depend on the matching fangd release line');
  }
  if (
    !depends.includes('fangd (>= ' + expected + ')') ||
    !depends.includes('fangd (<< ' + upper + ')')
  ) {
    throw new Error('Tauri deb must depend on the matching fangd release line');
  }
  console.log('Fang version sync OK: ' + expected);
}

function setVersion(version) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) {
    throw new Error('version must be MAJOR.MINOR.PATCH');
  }

  let text = read('Cargo.toml');
  text = replaceRequired(
    'workspace Cargo.toml',
    text,
    /(\[workspace\.package\][\s\S]*?\nversion = ")[^"]+(")/,
    '$1' + version + '$2'
  );
  write('Cargo.toml', text);

  text = replaceCargoPackageVersion(read('Cargo.lock'), 'fang-protocol', version);
  text = replaceCargoPackageVersion(text, 'fangd', version);
  write('Cargo.lock', text);

  const packageJson = JSON.parse(read('app/package.json'));
  packageJson.version = version;
  write('app/package.json', JSON.stringify(packageJson, null, 2) + '\n');

  const packageLock = JSON.parse(read('app/package-lock.json'));
  packageLock.version = version;
  packageLock.packages[''].version = version;
  write('app/package-lock.json', JSON.stringify(packageLock, null, 2) + '\n');

  text = read('app/src-tauri/Cargo.toml');
  text = replaceRequired(
    'Tauri Cargo.toml',
    text,
    /(\[package\][\s\S]*?\nversion = ")[^"]+(")/,
    '$1' + version + '$2'
  );
  write('app/src-tauri/Cargo.toml', text);

  text = replaceCargoPackageVersion(read('app/src-tauri/Cargo.lock'), 'fang', version);
  text = replaceCargoPackageVersion(text, 'fang-protocol', version);
  write('app/src-tauri/Cargo.lock', text);

  const tauri = JSON.parse(read('app/src-tauri/tauri.conf.json'));
  tauri.version = version;
  const [major, minor] = version.split('.').map(Number);
  tauri.bundle.linux.deb.depends = [
    'fangd (>= ' + version + ')',
    'fangd (<< ' + major + '.' + (minor + 1) + '.0)'
  ];
  write('app/src-tauri/tauri.conf.json', JSON.stringify(tauri, null, 2) + '\n');

  for (const name of ['packaging/rpm/fang.spec', 'packaging/rpm/fangd.spec']) {
    text = replaceRpmField(read(name), 'Version', version);
    if (name.endsWith('/fang.spec')) {
      text = replaceRpmMacro(text, 'fangd_upper', major + '.' + (minor + 1) + '.0');
    }
    write(name, text);
  }

  text = read('install.sh');
  text = replaceRequired(
    'release installer version',
    text,
    /^(readonly VERSION=')[^']+(')$/m,
    '$1' + version + '$2'
  );
  text = replaceRequired(
    'release installer tag',
    text,
    /^(readonly RELEASE_TAG='v)[^']+(')$/m,
    '$1' + version + '$2'
  );
  write('install.sh', text);

  console.log('Updated manifests and lockfiles to ' + version + '.');
  console.log('Update CHANGELOG.md, then run this script with check.');
}

const command = process.argv[2] ?? 'check';
if (command === 'check') {
  check();
} else if (command === 'set') {
  setVersion(process.argv[3] ?? '');
} else {
  throw new Error('usage: version.mjs [check | set VERSION]');
}
