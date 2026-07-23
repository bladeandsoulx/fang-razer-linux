import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const installer = path.join(root, 'install.sh');
const version = JSON.parse(fs.readFileSync(path.join(root, 'app/package.json'), 'utf8')).version;

function executable(file, text) {
  fs.writeFileSync(file, text, { mode: 0o755 });
}

function makeFixture({
  osRelease,
  arch = 'x86_64',
  euid = '1000',
  curlFailure = '',
  curlSignal = '',
  groups = 'home'
}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'fang-installer-test-'));
  const bin = path.join(dir, 'bin');
  const temporary = path.join(dir, 'tmp');
  const log = path.join(dir, 'commands.log');
  const releaseFile = path.join(dir, 'os-release');
  fs.mkdirSync(bin, { mode: 0o700 });
  fs.mkdirSync(temporary, { mode: 0o700 });
  fs.writeFileSync(log, '');
  fs.writeFileSync(releaseFile, osRelease);

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
  SHA256SUMS) printf '%064d  install.sh\\n' 0 > "$output" ;;
  *) printf '%s\\n' "$name" > "$output" ;;
esac
`
  );
  executable(
    path.join(bin, 'sudo'),
    `#!/usr/bin/env bash\nprintf 'sudo %s\\n' "$*" >> "\${FANG_TEST_LOG}"\nexit 0\n`
  );
  for (const command of [
    'systemctl',
    'dpkg',
    'dpkg-deb',
    'dpkg-query',
    'apt-get',
    'rpm',
    'dnf',
    'usermod'
  ]) {
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
    FANG_TEST_GROUPS: groups,
    FANG_TEST_HOME: path.join(dir, 'home'),
    FANG_TEST_LOG: log,
    HOME: path.join(dir, 'untrusted-home'),
    USER: 'untrusted-user',
    SUDO_USER: 'untrusted-sudo-user',
    NO_COLOR: '1'
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
