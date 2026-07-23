#!/usr/bin/env node

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

export function releaseNames(version) {
  return [
    'install.sh',
    'SHA256SUMS',
    `Fang_${version}_amd64.deb`,
    `fangd_${version}-1_amd64.deb`,
    `fang-${version}-1.x86_64.rpm`,
    `fangd-${version}-1.x86_64.rpm`
  ];
}

export function checksumNames(version) {
  return releaseNames(version).filter((name) => name !== 'SHA256SUMS');
}

function assertExactNames(actual, expected, label = 'inventory') {
  if (actual.length !== new Set(actual).size) {
    throw new Error(`${label} contains duplicate filenames`);
  }
  const sortedActual = [...actual].sort();
  const sortedExpected = [...expected].sort();
  if (
    sortedActual.length !== sortedExpected.length ||
    sortedActual.some((name, index) => name !== sortedExpected[index])
  ) {
    throw new Error(
      `${label} mismatch: expected ${sortedExpected.join(', ')}, got ${sortedActual.join(', ')}`
    );
  }
}

export function validateManifest(text, expectedNames) {
  if (!text.endsWith('\n') || text.endsWith('\n\n')) {
    throw new Error('checksum manifest must have exactly one final newline');
  }
  const names = text
    .slice(0, -1)
    .split('\n')
    .map((line) => {
      const match = line.match(/^([a-f0-9]{64})  ([^/]+)$/);
      if (!match) throw new Error(`malformed checksum line: ${line}`);
      return match[2];
    });
  assertExactNames(names, expectedNames, 'checksum manifest');
}

function commandFields(command, args) {
  return execFileSync(command, args, { encoding: 'utf8' }).trimEnd().split('\n');
}

function inspectDebDefault(file) {
  const [name, version, arch] = commandFields('dpkg-deb', [
    '-f',
    file,
    'Package',
    'Version',
    'Architecture'
  ]);
  return { name, version, arch };
}

function inspectRpmDefault(file) {
  const [name, epoch, version, release, arch] = commandFields('rpm', [
    '-qp',
    '--queryformat',
    '%{NAME}\n%{EPOCH}\n%{VERSION}\n%{RELEASE}\n%{ARCH}\n',
    file
  ]);
  return { name, epoch, version, release, arch };
}

function assertMetadata(actual, expected, file) {
  for (const [field, value] of Object.entries(expected)) {
    const accepted =
      field === 'epoch' && value === '0'
        ? actual[field] === '0' || actual[field] === '(none)' || actual[field] === ''
        : actual[field] === value;
    if (!accepted) {
      throw new Error(
        `${path.basename(file)} metadata ${field}: expected ${value}, got ${actual[field]}`
      );
    }
  }
}

function sha256(file) {
  const hash = crypto.createHash('sha256');
  hash.update(fs.readFileSync(file));
  return hash.digest('hex');
}

export function stageRelease({
  version,
  debDir,
  rpmDir,
  outputDir,
  installer,
  inspectDeb = inspectDebDefault,
  inspectRpm = inspectRpmDefault
}) {
  const names = releaseNames(version);
  const packageNames = names.slice(2);
  const expectedDebs = packageNames.filter((name) => name.endsWith('.deb'));
  const expectedRpms = packageNames.filter((name) => name.endsWith('.rpm'));
  assertExactNames(
    fs.readdirSync(debDir).filter((name) => name.endsWith('.deb')),
    expectedDebs,
    'DEB artifact directory'
  );
  assertExactNames(
    fs.readdirSync(rpmDir).filter((name) => name.endsWith('.rpm')),
    expectedRpms,
    'RPM artifact directory'
  );

  const fangDeb = path.join(debDir, expectedDebs[0]);
  const fangdDeb = path.join(debDir, expectedDebs[1]);
  const fangRpm = path.join(rpmDir, expectedRpms[0]);
  const fangdRpm = path.join(rpmDir, expectedRpms[1]);
  assertMetadata(inspectDeb(fangDeb), { name: 'fang', version, arch: 'amd64' }, fangDeb);
  assertMetadata(
    inspectDeb(fangdDeb),
    { name: 'fangd', version: `${version}-1`, arch: 'amd64' },
    fangdDeb
  );
  assertMetadata(
    inspectRpm(fangRpm),
    { name: 'fang', epoch: '0', version, release: '1', arch: 'x86_64' },
    fangRpm
  );
  assertMetadata(
    inspectRpm(fangdRpm),
    { name: 'fangd', epoch: '0', version, release: '1', arch: 'x86_64' },
    fangdRpm
  );
  if (!fs.statSync(installer).isFile()) throw new Error('installer is not a regular file');

  if (fs.existsSync(outputDir)) {
    throw new Error(`release output already exists: ${outputDir}`);
  }
  fs.mkdirSync(outputDir, { recursive: false, mode: 0o700 });
  const sources = new Map([
    ['install.sh', installer],
    [path.basename(fangDeb), fangDeb],
    [path.basename(fangdDeb), fangdDeb],
    [path.basename(fangRpm), fangRpm],
    [path.basename(fangdRpm), fangdRpm]
  ]);
  for (const name of checksumNames(version)) {
    fs.copyFileSync(sources.get(name), path.join(outputDir, name));
  }
  const manifest = checksumNames(version)
    .map((name) => `${sha256(path.join(outputDir, name))}  ${name}\n`)
    .join('');
  validateManifest(manifest, checksumNames(version));
  fs.writeFileSync(path.join(outputDir, 'SHA256SUMS'), manifest, { mode: 0o600 });
  assertExactNames(fs.readdirSync(outputDir), names, 'staged release');
}

function main(args) {
  if (args.length !== 6 || args[0] !== 'stage') {
    throw new Error(
      'usage: release-contract.mjs stage VERSION DEB_DIR RPM_DIR OUTPUT_DIR INSTALLER'
    );
  }
  const [, version, debDir, rpmDir, outputDir, installer] = args;
  stageRelease({ version, debDir, rpmDir, outputDir, installer });
  console.log(`Staged immutable Fang v${version} release in ${outputDir}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
