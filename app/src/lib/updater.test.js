import assert from 'node:assert/strict';
import test from 'node:test';
import { checkForUpdate, isNewerVersion } from './updater.js';

test('compares semantic versions numerically', () => {
  assert.equal(isNewerVersion('v0.10.0', '0.9.9'), true);
  assert.equal(isNewerVersion('v0.8.1', '0.8.1'), false);
  assert.equal(isNewerVersion('v0.8.1', '0.8.2'), false);
  assert.equal(isNewerVersion('v1.0.0', '0.8.2-sim'), true);
});

test('returns a newer published GitHub release', async () => {
  const fetchImpl = async () => ({
    ok: true,
    status: 200,
    async json() {
      return {
        tag_name: 'v0.9.0',
        html_url: 'https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.9.0'
      };
    }
  });

  assert.deepEqual(await checkForUpdate('0.8.2', fetchImpl), {
    available: true,
    installedVersion: '0.8.2',
    latestVersion: '0.9.0',
    releaseUrl: 'https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.9.0'
  });
});

test('rejects failed and malformed release responses', async () => {
  await assert.rejects(
    checkForUpdate('0.8.2', async () => ({ ok: false, status: 403 })),
    /GitHub release check failed \(403\)/
  );
  await assert.rejects(
    checkForUpdate('0.8.2', async () => ({
      ok: true,
      status: 200,
      async json() {
        return { tag_name: 'latest' };
      }
    })),
    /invalid Fang version/
  );
});
