import assert from 'node:assert/strict';
import test from 'node:test';
import { createUiSettingsCommitter } from './ui-settings.js';

test('publishes settings only after the backend confirms them', async () => {
  const published = [];
  let resolveApply;
  const pendingApply = new Promise((resolve) => {
    resolveApply = resolve;
  });
  const committer = createUiSettingsCommitter(
    { autostart: false, close_to_tray: true },
    (value) => published.push(value)
  );

  const saving = committer.save(
    { autostart: true, close_to_tray: true },
    () => pendingApply
  );
  assert.deepEqual(published, []);

  resolveApply({ autostart: true, close_to_tray: true });
  await saving;
  assert.deepEqual(published, [{ autostart: true, close_to_tray: true }]);
});

test('restores the last confirmed value after a backend error', async () => {
  const published = [];
  const committer = createUiSettingsCommitter(
    { autostart: false, close_to_tray: true },
    (value) => published.push(value)
  );

  await assert.rejects(
    committer.save({ autostart: true, close_to_tray: true }, async () => {
      throw new Error('autostart failed');
    }),
    /autostart failed/
  );
  assert.deepEqual(published, [{ autostart: false, close_to_tray: true }]);
});
