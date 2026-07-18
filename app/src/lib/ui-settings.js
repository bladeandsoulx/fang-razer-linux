/**
 * Keep the UI store on the last backend-confirmed settings. The publisher is
 * intentionally called only after `apply` succeeds; failures republish the
 * previous value so one-way checkbox props snap back deterministically.
 */
export function createUiSettingsCommitter(initial, publish) {
  let confirmed = { ...initial };

  function confirm(settings) {
    confirmed = { ...settings };
    publish({ ...confirmed });
    return { ...confirmed };
  }

  async function save(next, apply) {
    try {
      const saved = await apply({ ...next });
      return confirm(saved ?? next);
    } catch (error) {
      publish({ ...confirmed });
      throw error;
    }
  }

  return { confirm, save };
}
