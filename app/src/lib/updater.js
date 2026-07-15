const REPOSITORY = 'https://github.com/bladeandsoulx/fang-razer-linux';
const LATEST_RELEASE_API =
  'https://api.github.com/repos/bladeandsoulx/fang-razer-linux/releases/latest';

function parseVersion(value) {
  const match = String(value)
    .trim()
    .match(/^v?(\d+)\.(\d+)\.(\d+)(?:[-+][0-9A-Za-z.-]+)?$/);

  if (!match) throw new Error(`invalid Fang version: ${value}`);

  return {
    version: `${match[1]}.${match[2]}.${match[3]}`,
    parts: match.slice(1, 4).map(Number)
  };
}

export function isNewerVersion(candidate, installed) {
  const next = parseVersion(candidate).parts;
  const current = parseVersion(installed).parts;

  for (let i = 0; i < next.length; i += 1) {
    if (next[i] !== current[i]) return next[i] > current[i];
  }
  return false;
}

function trustedReleaseUrl(value) {
  try {
    const url = new URL(value);
    if (
      url.origin === 'https://github.com' &&
      url.pathname.startsWith('/bladeandsoulx/fang-razer-linux/releases/')
    ) {
      return url.href;
    }
  } catch {
    // Fall back to the repository's releases page below.
  }
  return `${REPOSITORY}/releases`;
}

/** Check the latest published (non-draft, non-prerelease) GitHub release. */
export async function checkForUpdate(installedVersion, fetchImpl = globalThis.fetch) {
  if (typeof fetchImpl !== 'function') throw new Error('network requests are unavailable');

  const response = await fetchImpl(LATEST_RELEASE_API, {
    cache: 'no-store',
    headers: { Accept: 'application/vnd.github+json' }
  });

  if (!response.ok) throw new Error(`GitHub release check failed (${response.status})`);

  const release = await response.json();
  const latest = parseVersion(release?.tag_name);
  const installed = parseVersion(installedVersion);

  return {
    available: isNewerVersion(latest.version, installed.version),
    installedVersion: installed.version,
    latestVersion: latest.version,
    releaseUrl: trustedReleaseUrl(release?.html_url)
  };
}
