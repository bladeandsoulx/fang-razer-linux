import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const installer = path.join(root, 'install.sh');
const version = JSON.parse(fs.readFileSync(path.join(root, 'app/package.json'), 'utf8')).version;
const releaseNames = [
  'install.sh',
  `Fang_${version}_amd64.deb`,
  `fangd_${version}-1_amd64.deb`,
  `fang-${version}-1.x86_64.rpm`,
  `fangd-${version}-1.x86_64.rpm`
];

function executable(file, text) {
  fs.writeFileSync(file, text, { mode: 0o755 });
}

function makeFixture({
  osRelease,
  arch = 'x86_64',
  euid = '1000',
  curlFailure = '',
  curlSignal = '',
  groups = 'home',
  manifestTransform = (value) => value,
  corruptAsset = '',
  metadata = {},
  installed = {}
}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'fang-installer-test-'));
  const bin = path.join(dir, 'bin');
  const temporary = path.join(dir, 'tmp');
  const assets = path.join(dir, 'assets');
  const log = path.join(dir, 'commands.log');
  const releaseFile = path.join(dir, 'os-release');
  fs.mkdirSync(bin, { mode: 0o700 });
  fs.mkdirSync(temporary, { mode: 0o700 });
  fs.mkdirSync(assets, { mode: 0o700 });
  fs.writeFileSync(log, '');
  fs.writeFileSync(releaseFile, osRelease);
  for (const name of releaseNames) {
    fs.writeFileSync(path.join(assets, name), `fixture:${name}\n`);
  }
  const manifest = releaseNames
    .map((name) => {
      const digest = createHash('sha256').update(fs.readFileSync(path.join(assets, name))).digest('hex');
      return `${digest}  ${name}\n`;
    })
    .join('');
  fs.writeFileSync(path.join(assets, 'SHA256SUMS'), manifestTransform(manifest));
  if (corruptAsset) {
    fs.appendFileSync(path.join(assets, corruptAsset), 'corrupt\n');
  }

  executable(
    path.join(bin, 'uname'),
    `#!/usr/bin/env bash
printf 'uname %s\\n' "$*" >> "\${FANG_TEST_LOG}"
printf '%s\\n' "\${FANG_TEST_ARCH}"
`
  );
  executable(
    path.join(bin, 'id'),
    `#!/usr/bin/env bash
printf 'id %s\\n' "$*" >> "\${FANG_TEST_LOG}"
case "\${1:-}" in
  -u) printf '1000\\n' ;;
  -un) printf 'home\\n' ;;
  -nG) printf '%s\\n' "\${FANG_TEST_GROUPS}" ;;
  *) exit 2 ;;
esac
`
  );
  executable(
    path.join(bin, 'getent'),
    `#!/usr/bin/env bash
printf 'getent %s\\n' "$*" >> "\${FANG_TEST_LOG}"
case "\${1:-}:\${2:-}" in
  passwd:home) printf 'home:x:1000:1000:Fang Test:%s:/bin/bash\\n' "\${FANG_TEST_HOME}" ;;
  group:fang) printf 'fang:x:987:\\n' ;;
  *) exit 2 ;;
esac
`
  );
  executable(
    path.join(bin, 'curl'),
    `#!/usr/bin/env bash
output=
url=
while (($#)); do
  case "$1" in
    --output) output="$2"; shift 2 ;;
    http*) url="$1"; shift ;;
    *) shift ;;
  esac
done
printf 'curl %s\\n' "$url" >> "\${FANG_TEST_LOG}"
name="\${url##*/}"
if [[ "$name" == "\${FANG_TEST_CURL_SIGNAL}" ]]; then
  kill -TERM "$PPID"
  exit 143
fi
if [[ "$name" == "\${FANG_TEST_CURL_FAILURE}" ]]; then
  printf 'partial' > "$output"
  exit 22
fi
case "$name" in
  *) cp "\${FANG_TEST_ASSET_DIR}/$name" "$output" ;;
esac
`
  );
  executable(
    path.join(bin, 'sudo'),
    `#!/usr/bin/env bash\nprintf 'sudo %s\\n' "$*" >> "\${FANG_TEST_LOG}"\nexit 0\n`
  );
  executable(
    path.join(bin, 'dpkg-deb'),
    `#!/usr/bin/env bash
file="$2"
field="$3"
case "\${file##*/}:$field" in
  Fang_*:Package) printf '%s\\n' "\${FANG_TEST_DEB_FANG_PACKAGE}" ;;
  Fang_*:Version) printf '%s\\n' "\${FANG_TEST_DEB_FANG_VERSION}" ;;
  Fang_*:Architecture) printf '%s\\n' "\${FANG_TEST_DEB_FANG_ARCH}" ;;
  fangd_*:Package) printf '%s\\n' "\${FANG_TEST_DEB_FANGD_PACKAGE}" ;;
  fangd_*:Version) printf '%s\\n' "\${FANG_TEST_DEB_FANGD_VERSION}" ;;
  fangd_*:Architecture) printf '%s\\n' "\${FANG_TEST_DEB_FANGD_ARCH}" ;;
  *) exit 2 ;;
esac
`
  );
  executable(
    path.join(bin, 'dpkg-query'),
    `#!/usr/bin/env bash
package="\${@: -1}"
case "$package" in
  fang) value="\${FANG_TEST_INSTALLED_FANG}" ;;
  fangd) value="\${FANG_TEST_INSTALLED_FANGD}" ;;
  *) exit 2 ;;
esac
[[ -n "$value" ]] || exit 1
if [[ "$value" == residual:* ]]; then
  printf 'config-files\\t%s\\n' "\${value#residual:}"
else
  printf 'install ok installed\\t%s\\n' "$value"
fi
`
  );
  executable(
    path.join(bin, 'rpm'),
    `#!/usr/bin/env bash
if [[ "$1" == "-qp" ]]; then
  file="\${@: -1}"
  case "\${file##*/}" in
    fang-*) printf '%s\\n%s\\n%s\\n%s\\n%s\\n' \
      "\${FANG_TEST_RPM_FANG_NAME}" "\${FANG_TEST_RPM_FANG_EPOCH}" \
      "\${FANG_TEST_RPM_FANG_VERSION}" "\${FANG_TEST_RPM_FANG_RELEASE}" \
      "\${FANG_TEST_RPM_FANG_ARCH}" ;;
    fangd-*) printf '%s\\n%s\\n%s\\n%s\\n%s\\n' \
      "\${FANG_TEST_RPM_FANGD_NAME}" "\${FANG_TEST_RPM_FANGD_EPOCH}" \
      "\${FANG_TEST_RPM_FANGD_VERSION}" "\${FANG_TEST_RPM_FANGD_RELEASE}" \
      "\${FANG_TEST_RPM_FANGD_ARCH}" ;;
    *) exit 2 ;;
  esac
elif [[ "$1" == "-q" ]]; then
  package="\${@: -1}"
  case "$package" in
    fang) value="\${FANG_TEST_INSTALLED_FANG}" ;;
    fangd) value="\${FANG_TEST_INSTALLED_FANGD}" ;;
    *) exit 2 ;;
  esac
  [[ -n "$value" ]] || exit 1
  printf '%s\\n' "$value"
elif [[ "$1" == "--eval" ]]; then
  printf 'rpm-vercmp %s %s\\n' "\${FANG_RPM_LEFT}" "\${FANG_RPM_RIGHT}" >> "\${FANG_TEST_LOG}"
  if [[ "\${FANG_RPM_LEFT}" == "\${FANG_RPM_RIGHT}" ]]; then
    printf '0\\n'
  else
    case "\${FANG_RPM_LEFT}:\${FANG_RPM_RIGHT}" in
      0:0.9.3-1:0:0.9.4-1) printf '%s\\n' -1 ;;
      0:0.9.4-1:0:0.9.3-1) printf '%s\\n' 1 ;;
      1:0.9.3-1:0:0.9.4-1) printf '%s\\n' 1 ;;
      0:0.9.4-2:0:0.9.4-1) printf '%s\\n' 1 ;;
      *) exit 2 ;;
    esac
  fi
else
  exit 2
fi
`
  );
  for (const command of ['systemctl', 'apt-get', 'dnf', 'usermod']) {
    executable(path.join(bin, command), '#!/usr/bin/env bash\nexit 0\n');
  }

  const env = {
    PATH: `${bin}:/usr/bin:/bin`,
    TMPDIR: temporary,
    FANG_INSTALLER_TESTING: '1',
    FANG_INSTALLER_TEST_EUID: euid,
    FANG_OS_RELEASE_FILE: releaseFile,
    FANG_TEST_ARCH: arch,
    FANG_TEST_CURL_FAILURE: curlFailure,
    FANG_TEST_CURL_SIGNAL: curlSignal,
    FANG_TEST_ASSET_DIR: assets,
    FANG_TEST_GROUPS: groups,
    FANG_TEST_HOME: path.join(dir, 'home'),
    FANG_TEST_LOG: log,
    HOME: path.join(dir, 'untrusted-home'),
    USER: 'untrusted-user',
    SUDO_USER: 'untrusted-sudo-user',
    NO_COLOR: '1',
    FANG_TEST_DEB_FANG_PACKAGE: metadata.debFangPackage ?? 'fang',
    FANG_TEST_DEB_FANG_VERSION: metadata.debFangVersion ?? version,
    FANG_TEST_DEB_FANG_ARCH: metadata.debFangArch ?? 'amd64',
    FANG_TEST_DEB_FANGD_PACKAGE: metadata.debFangdPackage ?? 'fangd',
    FANG_TEST_DEB_FANGD_VERSION: metadata.debFangdVersion ?? `${version}-1`,
    FANG_TEST_DEB_FANGD_ARCH: metadata.debFangdArch ?? 'amd64',
    FANG_TEST_RPM_FANG_NAME: metadata.rpmFangName ?? 'fang',
    FANG_TEST_RPM_FANG_EPOCH: metadata.rpmFangEpoch ?? '(none)',
    FANG_TEST_RPM_FANG_VERSION: metadata.rpmFangVersion ?? version,
    FANG_TEST_RPM_FANG_RELEASE: metadata.rpmFangRelease ?? '1',
    FANG_TEST_RPM_FANG_ARCH: metadata.rpmFangArch ?? 'x86_64',
    FANG_TEST_RPM_FANGD_NAME: metadata.rpmFangdName ?? 'fangd',
    FANG_TEST_RPM_FANGD_EPOCH: metadata.rpmFangdEpoch ?? '0',
    FANG_TEST_RPM_FANGD_VERSION: metadata.rpmFangdVersion ?? version,
    FANG_TEST_RPM_FANGD_RELEASE: metadata.rpmFangdRelease ?? '1',
    FANG_TEST_RPM_FANGD_ARCH: metadata.rpmFangdArch ?? 'x86_64',
    FANG_TEST_INSTALLED_FANG: installed.fang ?? '',
    FANG_TEST_INSTALLED_FANGD: installed.fangd ?? ''
  };
  return {
    dir,
    env,
    log,
    temporary,
    run() {
      return spawnSync('bash', [installer], { env, encoding: 'utf8' });
    },
    commands() {
      return fs.readFileSync(log, 'utf8');
    },
    cleanup() {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  };
}

const directPlatforms = [
  ['Ubuntu 22.04', 'ID=ubuntu\nVERSION_ID="22.04"\nVERSION_CODENAME=jammy\n', 'DEB'],
  ['Ubuntu 24.04', 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n', 'DEB'],
  ['Debian 12', 'ID=debian\nVERSION_ID="12"\nVERSION_CODENAME=bookworm\n', 'DEB'],
  ['Debian 13', 'ID=debian\nVERSION_ID="13"\nVERSION_CODENAME=trixie\n', 'DEB'],
  ['Fedora 43', 'ID=fedora\nVERSION_ID="43"\nPLATFORM_ID="platform:f43"\n', 'RPM'],
  ['Fedora 44', 'ID=fedora\nVERSION_ID="44"\nPLATFORM_ID="platform:f44"\n', 'RPM']
];

for (const [label, osRelease, family] of directPlatforms) {
  test(`detects ${label}`, () => {
    const fixture = makeFixture({ osRelease });
    const result = fixture.run();
    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.match(result.stdout, new RegExp(`Detected: linux \\(${label.replace('.', '\\.')}\\)`));
    const commands = fixture.commands();
    assert.match(commands, new RegExp(`/releases/download/v${version}/SHA256SUMS`));
    assert.match(commands, family === 'DEB' ? /\.deb/ : /\.rpm/);
    fixture.cleanup();
  });
}

const derivatives = [
  [
    'zorin',
    'Ubuntu 24.04',
    'ID=zorin\nID_LIKE="ubuntu debian"\nVERSION_ID="18.1"\nUBUNTU_CODENAME=noble\n'
  ],
  [
    'linuxmint',
    'Ubuntu 22.04',
    'ID=linuxmint\nID_LIKE="ubuntu debian"\nVERSION_ID="21.3"\nUBUNTU_CODENAME=jammy\n'
  ],
  [
    'pop',
    'Ubuntu 24.04',
    'ID=pop\nID_LIKE="ubuntu debian"\nVERSION_ID="24.04"\nUBUNTU_CODENAME=noble\n'
  ],
  [
    'devuan',
    'Debian 12',
    'ID=devuan\nID_LIKE=debian\nVERSION_ID="5"\nVERSION_CODENAME=bookworm\n'
  ],
  [
    'ultramarine',
    'Fedora 44',
    'ID=ultramarine\nID_LIKE="fedora"\nVERSION_ID="40"\nPLATFORM_ID="platform:f44"\n'
  ]
];

for (const [id, base, osRelease] of derivatives) {
  test(`detects supported ${id} derivative`, () => {
    const fixture = makeFixture({ osRelease });
    const result = fixture.run();
    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.match(result.stdout, new RegExp(`${id} → ${base.replace('.', '\\.')} family`));
    assert.match(result.stdout, /compatible-family, not release-tested directly/);
    fixture.cleanup();
  });
}

test('refuses root and unsupported architecture before sudo', () => {
  for (const [options, message] of [
    [{ euid: '0' }, /without sudo/],
    [{ arch: 'aarch64' }, /only x86_64/],
    [{ arch: 'amd64' }, /only x86_64/]
  ]) {
    const fixture = makeFixture({
      osRelease: 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n',
      ...options
    });
    const result = fixture.run();
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, message);
    assert.doesNotMatch(fixture.commands(), /^sudo /m);
    fixture.cleanup();
  }
});

test('refuses malformed, duplicate, unsupported, and conflicting platform data', () => {
  const cases = [
    'ID=ubuntu\nID=debian\nVERSION_ID="24.04"\n',
    'ID="$(touch /tmp/no)"\nVERSION_ID="24.04"\n',
    'ID=ubuntu\nVERSION_ID="26.04"\nVERSION_CODENAME=questing\n',
    'ID=zorin\nID_LIKE="ubuntu debian"\nUBUNTU_CODENAME=questing\n',
    'ID=mystery\nVERSION_ID="1"\n',
    'ID=hybrid\nID_LIKE="ubuntu fedora"\nUBUNTU_CODENAME=noble\nPLATFORM_ID=platform:f44\n'
  ];
  for (const osRelease of cases) {
    const fixture = makeFixture({ osRelease });
    const result = fixture.run();
    assert.notEqual(result.status, 0, osRelease);
    assert.doesNotMatch(fixture.commands(), /^sudo /m);
    fixture.cleanup();
  }
});

test('downloads only the selected exact pinned package pair', () => {
  const fixture = makeFixture({
    osRelease: 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n'
  });
  const result = fixture.run();
  assert.equal(result.status, 0, result.stdout + result.stderr);
  const commands = fixture.commands();
  assert.match(commands, new RegExp(`/v${version}/Fang_${version}_amd64\\.deb`));
  assert.match(commands, new RegExp(`/v${version}/fangd_${version}-1_amd64\\.deb`));
  assert.doesNotMatch(commands, /\.rpm/);
  fixture.cleanup();
});

test('rejects malformed checksum manifests before sudo', () => {
  const transformations = [
    (value) => value.split('\n').slice(1).join('\n'),
    (value) => `${value}${value.split('\n')[0]}\n`,
    (value) => value.replace(/^[a-f0-9]{64}/, 'BAD'),
    (value) => value.replace('install.sh', '../install.sh'),
    (value) => `${value}${'b'.repeat(64)}  seventh.asset\n`,
    (value) => value.trimEnd()
  ];
  for (const manifestTransform of transformations) {
    const fixture = makeFixture({
      osRelease: 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n',
      manifestTransform
    });
    const result = fixture.run();
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /checksum manifest/i);
    assert.doesNotMatch(fixture.commands(), /^sudo /m);
    fixture.cleanup();
  }
});

test('rejects a wrong checksum for every selected package before sudo', () => {
  for (const [osRelease, names] of [
    [
      'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n',
      [`Fang_${version}_amd64.deb`, `fangd_${version}-1_amd64.deb`]
    ],
    [
      'ID=fedora\nVERSION_ID="44"\nPLATFORM_ID="platform:f44"\n',
      [`fang-${version}-1.x86_64.rpm`, `fangd-${version}-1.x86_64.rpm`]
    ]
  ]) {
    for (const corruptAsset of names) {
      const fixture = makeFixture({ osRelease, corruptAsset });
      const result = fixture.run();
      assert.notEqual(result.status, 0, corruptAsset);
      assert.match(result.stderr + result.stdout, /checksum/i);
      assert.doesNotMatch(fixture.commands(), /^sudo /m);
      fixture.cleanup();
    }
  }
});

test('rejects every wrong DEB metadata field before sudo', () => {
  const cases = [
    { debFangPackage: 'other' },
    { debFangVersion: `${version}-1` },
    { debFangArch: 'arm64' },
    { debFangdPackage: 'otherd' },
    { debFangdVersion: version },
    { debFangdArch: 'arm64' }
  ];
  for (const metadata of cases) {
    const fixture = makeFixture({
      osRelease: 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n',
      metadata
    });
    const result = fixture.run();
    assert.notEqual(result.status, 0, JSON.stringify(metadata));
    assert.match(result.stderr, /metadata/i);
    assert.doesNotMatch(fixture.commands(), /^sudo /m);
    fixture.cleanup();
  }
});

test('rejects every wrong RPM metadata field before sudo', () => {
  const cases = [
    { rpmFangName: 'other' },
    { rpmFangEpoch: '1' },
    { rpmFangVersion: '0.9.3' },
    { rpmFangRelease: '2' },
    { rpmFangArch: 'aarch64' },
    { rpmFangdName: 'otherd' },
    { rpmFangdEpoch: '1' },
    { rpmFangdVersion: '0.9.3' },
    { rpmFangdRelease: '2' },
    { rpmFangdArch: 'aarch64' }
  ];
  for (const metadata of cases) {
    const fixture = makeFixture({
      osRelease: 'ID=fedora\nVERSION_ID="44"\nPLATFORM_ID="platform:f44"\n',
      metadata
    });
    const result = fixture.run();
    assert.notEqual(result.status, 0, JSON.stringify(metadata));
    assert.match(result.stderr, /metadata/i);
    assert.doesNotMatch(fixture.commands(), /^sudo /m);
    fixture.cleanup();
  }
});

test('DEB installed-version policy rejects downgrades and keeps one pair transaction', () => {
  const osRelease = 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n';
  for (const installed of [
    { fang: '', fangd: '' },
    { fang: '0.9.3', fangd: `${version}-1` },
    { fang: version, fangd: '0.9.3-1' },
    { fang: `residual:${version}`, fangd: `residual:${version}-1` }
  ]) {
    const fixture = makeFixture({ osRelease, installed });
    const result = fixture.run();
    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.match(
      fixture.commands(),
      new RegExp(
        `^sudo apt-get install .*${path.sep}fangd_${version}-1_amd64\\.deb .*${path.sep}Fang_${version}_amd64\\.deb$`,
        'm'
      )
    );
    fixture.cleanup();
  }

  const equal = makeFixture({
    osRelease,
    installed: { fang: version, fangd: `${version}-1` }
  });
  const equalResult = equal.run();
  assert.equal(equalResult.status, 0, equalResult.stdout + equalResult.stderr);
  assert.doesNotMatch(equal.commands(), /^sudo (?:apt-get|dnf) /m);
  equal.cleanup();

  for (const installed of [
    { fang: '0.9.5', fangd: '' },
    { fang: '', fangd: `${version}-2` },
    { fang: '1:0.9.3-1', fangd: `${version}-1` }
  ]) {
    const fixture = makeFixture({ osRelease, installed });
    const result = fixture.run();
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /refusing downgrade/i);
    assert.doesNotMatch(fixture.commands(), /^sudo /m);
    fixture.cleanup();
  }
});

test('RPM installed-version policy uses EVR and rejects ambiguous records', () => {
  const osRelease = 'ID=fedora\nVERSION_ID="44"\nPLATFORM_ID="platform:f44"\n';
  for (const installed of [
    { fang: '', fangd: '' },
    { fang: '0:0.9.3-1', fangd: `0:${version}-1` },
    { fang: `0:${version}-1`, fangd: '0:0.9.3-1' }
  ]) {
    const fixture = makeFixture({ osRelease, installed });
    const result = fixture.run();
    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.match(
      fixture.commands(),
      new RegExp(
        `^sudo dnf install .*${path.sep}fangd-${version}-1\\.x86_64\\.rpm .*${path.sep}fang-${version}-1\\.x86_64\\.rpm$`,
        'm'
      )
    );
    fixture.cleanup();
  }

  const equal = makeFixture({
    osRelease,
    installed: { fang: `0:${version}-1`, fangd: `0:${version}-1` }
  });
  const equalResult = equal.run();
  assert.equal(equalResult.status, 0, equalResult.stdout + equalResult.stderr);
  assert.doesNotMatch(equal.commands(), /^sudo (?:apt-get|dnf) /m);
  equal.cleanup();

  for (const installed of [
    { fang: '1:0.9.3-1', fangd: '' },
    { fang: '', fangd: `0:${version}-2` },
    { fang: `0:${version}-1\n0:${version}-1`, fangd: '' }
  ]) {
    const fixture = makeFixture({ osRelease, installed });
    const result = fixture.run();
    assert.notEqual(result.status, 0);
    assert.doesNotMatch(fixture.commands(), /^sudo /m);
    fixture.cleanup();
  }
});

test('download failure removes partial and temporary files', () => {
  const fixture = makeFixture({
    osRelease: 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n',
    curlFailure: `fangd_${version}-1_amd64.deb`
  });
  const result = fixture.run();
  assert.notEqual(result.status, 0);
  assert.deepEqual(fs.readdirSync(fixture.temporary), []);
  assert.doesNotMatch(fixture.commands(), /^sudo /m);
  fixture.cleanup();
});

test('signal cleanup removes temporary and partial files', () => {
  const fixture = makeFixture({
    osRelease: 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n',
    curlSignal: `Fang_${version}_amd64.deb`
  });
  const result = fixture.run();
  assert.notEqual(result.status, 0);
  assert.deepEqual(fs.readdirSync(fixture.temporary), []);
  assert.doesNotMatch(fixture.commands(), /^sudo /m);
  fixture.cleanup();
});

test('one final main call is the only top-level executable action', () => {
  const source = fs.readFileSync(installer, 'utf8');
  const executableLines = source
    .split('\n')
    .filter((line) => line.length > 0 && !line.startsWith('#') && !line.startsWith('main()') && !line.startsWith('}'));
  assert.equal(source.trimEnd().split('\n').at(-1), 'main "$@"');
  assert.equal(executableLines.at(-1), 'main "$@"');
});

test('every parseable line-boundary truncation is fail-closed', () => {
  const source = fs.readFileSync(installer, 'utf8');
  const lines = source.split('\n');
  const prefixes = fs.mkdtempSync(path.join(os.tmpdir(), 'fang-installer-prefixes-'));
  for (let length = 1; length < lines.length - 1; length += 1) {
    const prefix = `${lines.slice(0, length).join('\n')}\n`;
    fs.writeFileSync(path.join(prefixes, String(length).padStart(6, '0')), prefix);
  }
  const fixture = makeFixture({
    osRelease: 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n'
  });
  const driver = String.raw`
set -e
count=0
for candidate in "$1"/*; do
  if source "$candidate" 2>/dev/null; then
    count=$((count + 1))
  fi
done
printf '%s\n' "$count"
`;
  const result = spawnSync('bash', ['-c', driver, 'bash', prefixes], {
    env: fixture.env,
    encoding: 'utf8'
  });
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.ok(Number(result.stdout.trim()) > 20);
  assert.equal(fixture.commands(), '');
  assert.deepEqual(fs.readdirSync(fixture.temporary), []);
  fixture.cleanup();
  fs.rmSync(prefixes, { recursive: true, force: true });
});
