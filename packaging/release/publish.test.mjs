import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import { releaseNames } from './release-contract.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const publisher = path.join(root, 'packaging/release/publish.sh');
const workflow = path.join(root, '.github/workflows/release.yml');
const version = '0.9.4';
const tag = `v${version}`;

function executable(file, source) {
  fs.writeFileSync(file, source, { mode: 0o755 });
}

function makeFixture({ badDigest = false } = {}) {
  const fixture = fs.mkdtempSync(path.join(os.tmpdir(), 'fang-publish-test-'));
  const bin = path.join(fixture, 'bin');
  const releaseDir = path.join(fixture, 'release');
  const log = path.join(fixture, 'gh.log');
  const draftJson = path.join(fixture, 'draft.json');
  const publishedJson = path.join(fixture, 'published.json');
  const publishedMarker = path.join(fixture, 'published');
  fs.mkdirSync(bin);
  fs.symlinkSync(process.execPath, path.join(bin, 'node'));
  fs.mkdirSync(releaseDir);
  fs.writeFileSync(log, '');

  const assets = releaseNames(version).map((name, index) => {
    const file = path.join(releaseDir, name);
    fs.writeFileSync(file, `asset ${index}: ${name}\n`);
    const digest = createHash('sha256').update(fs.readFileSync(file)).digest('hex');
    return {
      name,
      size: fs.statSync(file).size,
      digest: `sha256:${badDigest && index === 0 ? '0'.repeat(64) : digest}`
    };
  });
  fs.writeFileSync(
    draftJson,
    JSON.stringify({ id: 42, tag_name: tag, draft: true, prerelease: false, immutable: false, assets })
  );
  fs.writeFileSync(
    publishedJson,
    JSON.stringify({
      id: 42,
      tag_name: tag,
      draft: false,
      prerelease: false,
      immutable: true,
      assets
    })
  );

  executable(
    path.join(bin, 'gh'),
    `#!/usr/bin/env bash
printf '%s\\n' "$*" >> "\${FANG_TEST_GH_LOG}"
joined="$*"
case "$joined" in
  *immutable-releases*)
    printf 'true\\n'
    ;;
  *releases/tags/*)
    printf 'HTTP/2 404 Not Found\\n'
    exit 1
    ;;
  *uploads.github.com*)
    printf '{}\\n'
    ;;
  *"--method POST"*"/releases")
    printf '42\\n'
    ;;
  *"--method PATCH"*)
    : > "\${FANG_TEST_PUBLISHED_MARKER}"
    cat "\${FANG_TEST_PUBLISHED_JSON}"
    ;;
  *"/releases/latest")
    cat "\${FANG_TEST_PUBLISHED_JSON}"
    ;;
  *"/releases/42")
    if [[ -f "\${FANG_TEST_PUBLISHED_MARKER}" ]]; then
      cat "\${FANG_TEST_PUBLISHED_JSON}"
    else
      cat "\${FANG_TEST_DRAFT_JSON}"
    fi
    ;;
  *) exit 2 ;;
esac
`
  );
  executable(
    path.join(bin, 'curl'),
    `#!/usr/bin/env bash
for argument in "$@"; do
  case "$argument" in
    https://uploads.github.com/*)
      printf '%s\\n' "$argument" >> "\${FANG_TEST_GH_LOG}"
      ;;
  esac
done
printf '{}\\n'
`
  );

  const env = {
    PATH: `${bin}:/usr/bin:/bin`,
    GITHUB_REPOSITORY: 'bladeandsoulx/fang-razer-linux',
    GITHUB_SHA: 'a'.repeat(40),
    GITHUB_TOKEN: 'contents-token',
    IMMUTABLE_RELEASES_TOKEN: 'administration-read-token',
    FANG_TEST_GH_LOG: log,
    FANG_TEST_DRAFT_JSON: draftJson,
    FANG_TEST_PUBLISHED_JSON: publishedJson,
    FANG_TEST_PUBLISHED_MARKER: publishedMarker
  };
  return {
    bin,
    releaseDir,
    log,
    run() {
      return spawnSync('bash', [publisher, releaseDir, tag], { env, encoding: 'utf8' });
    },
    cleanup() {
      fs.rmSync(fixture, { recursive: true, force: true });
    }
  };
}

test('publication fixture supplies Node inside its isolated PATH', () => {
  const fixture = makeFixture();
  try {
    assert.equal(
      fs.realpathSync(path.join(fixture.bin, 'node')),
      fs.realpathSync(process.execPath)
    );
  } finally {
    fixture.cleanup();
  }
});

test('publisher creates, validates, and publishes one six-asset release', () => {
  const fixture = makeFixture();
  const result = fixture.run();
  assert.equal(result.status, 0, result.stdout + result.stderr);
  const log = fs.readFileSync(fixture.log, 'utf8');
  assert.equal((log.match(/uploads\.github\.com/g) ?? []).length, 6);
  assert.equal((log.match(/--method POST .*\/releases$/gm) ?? []).length, 1);
  assert.equal((log.match(/--method PATCH/g) ?? []).length, 1);
  assert.match(log, /immutable-releases/);
  assert.match(log, /releases\/latest/);
  assert.doesNotMatch(log, /--method DELETE|clobber/);
  fixture.cleanup();
});

test('publisher leaves its draft unpublished when remote validation fails', () => {
  const fixture = makeFixture({ badDigest: true });
  const result = fixture.run();
  assert.notEqual(result.status, 0);
  const log = fs.readFileSync(fixture.log, 'utf8');
  assert.equal((log.match(/--method POST .*\/releases$/gm) ?? []).length, 1);
  assert.doesNotMatch(log, /--method PATCH/);
  assert.doesNotMatch(log, /--method DELETE/);
  fixture.cleanup();
});

test('release workflow has per-tag concurrency and exact publication gates', () => {
  const source = fs.readFileSync(workflow, 'utf8');
  assert.match(source, /group: release-\$\{\{ github\.ref_name \}\}/);
  assert.match(source, /cancel-in-progress: false/);
  assert.match(source, /IMMUTABLE_RELEASES_TOKEN: \$\{\{ secrets\.IMMUTABLE_RELEASES_TOKEN \}\}/);
  assert.match(source, /release-contract\.mjs stage/);
  assert.match(source, /packaging\/release\/publish\.sh/);
  assert.doesNotMatch(source, /softprops\/action-gh-release|create four-package draft/);
});

test('publisher names six explicit uploads and never mutates an existing release', () => {
  const source = fs.readFileSync(publisher, 'utf8');
  for (const name of [
    'install.sh',
    'SHA256SUMS',
    '"Fang_${VERSION}_amd64.deb"',
    '"fangd_${VERSION}-1_amd64.deb"',
    '"fang-${VERSION}-1.x86_64.rpm"',
    '"fangd-${VERSION}-1.x86_64.rpm"'
  ]) {
    assert.ok(source.includes(`upload_asset "$release_id" ${name}`), name);
  }
  assert.match(source, /X-GitHub-Api-Version: 2026-03-10/);
  assert.match(source, /make_latest=true/);
  assert.doesNotMatch(source, /--clobber|--method DELETE|release delete|\*\.deb|\*\.rpm/);
});

test('publisher uses the documented release-upload origin without CLI host rewriting', () => {
  const source = fs.readFileSync(publisher, 'utf8');
  assert.match(source, /https:\/\/uploads\.github\.com/);
  assert.doesNotMatch(source, /--hostname uploads\.github\.com/);
});
