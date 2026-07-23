import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const verifierPath = path.join(root, 'packaging/deb/verify.sh');
const workflows = [
  path.join(root, '.github/workflows/ci.yml'),
  path.join(root, '.github/workflows/release.yml')
];

test('DEB verifier owns the exact package and metadata contract', () => {
  assert.ok(fs.existsSync(verifierPath));
  const source = fs.readFileSync(verifierPath, 'utf8');
  assert.match(source, /Fang_\$\{VERSION\}_amd64\.deb/);
  assert.match(source, /fangd_\$\{VERSION\}-1_amd64\.deb/);
  assert.match(source, /expected exactly two DEBs/);
  for (const field of ['Package', 'Version', 'Architecture']) {
    assert.match(source, new RegExp(`dpkg-deb -f .* ${field}`));
  }
  assert.match(source, /fangd \(>= \$VERSION\)/);
  assert.match(source, /fangd \(<< \$FANGD_UPPER\)/);
});

test('DEB verifier covers install, runtime, integrity, and removal lifecycle', () => {
  const source = fs.readFileSync(verifierPath, 'utf8');
  assert.match(source, /apt-get install -y "\$fangd" "\$fang"/);
  assert.match(source, /getent group fang/);
  assert.match(source, /systemd-analyze verify .*fangd\.service/);
  assert.match(source, /packaging\/rpm\/mock_smoke\.py/);
  assert.match(source, /ldd \/usr\/bin\/fang/);
  assert.match(source, /dbus-run-session -- timeout .*xvfb-run -a \/usr\/bin\/fang/);
  assert.match(source, /dpkg -V fangd fang/);
  assert.match(source, /dpkg-query -L fang fangd/);
  assert.match(source, /apt-get remove -y fang fangd/);
  assert.match(source, /packaged file remains after removal/);
});

test('CI and release test one DEB pair on all four supported bases', () => {
  const images = ['ubuntu:22.04', 'ubuntu:24.04', 'debian:12', 'debian:13'];
  for (const workflow of workflows) {
    const source = fs.readFileSync(workflow, 'utf8');
    for (const image of images) assert.match(source, new RegExp(image.replace('.', '\\.')));
    assert.match(source, /packaging\/deb\/verify\.sh target\/deb-dist/);
    assert.match(source, /name: fang-debs/);
  }
});

test('CI exposes source, installer, release, frontend, Rust, and RPM gates', () => {
  const source = fs.readFileSync(workflows[0], 'utf8');
  assert.match(source, /node app\/scripts\/version\.mjs check/);
  assert.match(source, /packaging\/installer\/installer\.test\.mjs/);
  assert.match(source, /packaging\/release\/release-contract\.test\.mjs/);
  assert.match(source, /shellcheck install\.sh/);
  assert.match(source, /npm test/);
  assert.match(source, /cargo test --workspace/);
  assert.match(source, /packaging\/rpm\/verify\.sh target\/rpm-dist/);
});
