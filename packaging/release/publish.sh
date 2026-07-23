#!/usr/bin/env bash
set -euo pipefail

fatal() {
  printf 'release publication failed: %s\n' "$*" >&2
  exit 1
}

cleanup() {
  if [[ -n ${PUBLICATION_TEMPORARY:-} && -d ${PUBLICATION_TEMPORARY:-} ]]; then
    rm -rf -- "$PUBLICATION_TEMPORARY"
  fi
}

validate_release_json() {
  local json_file=$1
  local expected_state=$2

  node - "$RELEASE_DIR" "$json_file" "$TAG" "$expected_state" <<'NODE'
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const [releaseDir, jsonFile, tag, expectedState] = process.argv.slice(2);
const release = JSON.parse(fs.readFileSync(jsonFile, 'utf8'));
const names = fs.readdirSync(releaseDir).sort();
if (release.tag_name !== tag) throw new Error(`remote tag mismatch: ${release.tag_name}`);
if (release.prerelease !== false) throw new Error('release unexpectedly marked prerelease');
if (expectedState === 'draft') {
  if (release.draft !== true) throw new Error('release is not a draft before publication');
} else {
  if (release.draft !== false) throw new Error('release remained a draft');
  if (release.immutable !== true) throw new Error('published release is not immutable');
}
if (!Array.isArray(release.assets) || release.assets.length !== names.length) {
  throw new Error('remote release asset count mismatch');
}
const remote = new Map();
for (const asset of release.assets) {
  if (remote.has(asset.name)) throw new Error(`duplicate remote asset: ${asset.name}`);
  remote.set(asset.name, asset);
}
for (const name of names) {
  const file = path.join(releaseDir, name);
  const asset = remote.get(name);
  if (!asset) throw new Error(`missing remote asset: ${name}`);
  const size = fs.statSync(file).size;
  const digest = `sha256:${crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex')}`;
  if (asset.size !== size) throw new Error(`remote size mismatch for ${name}`);
  if (asset.digest !== digest) throw new Error(`remote digest mismatch for ${name}`);
}
if (!remote.has('install.sh')) throw new Error('remote release has no installer');
NODE
}

upload_asset() {
  local release_id=$1
  local name=$2

  GH_TOKEN=$GITHUB_TOKEN gh api \
    --hostname uploads.github.com \
    --method POST \
    -H 'Accept: application/vnd.github+json' \
    -H 'X-GitHub-Api-Version: 2026-03-10' \
    -H 'Content-Type: application/octet-stream' \
    --input "$RELEASE_DIR/$name" \
    "$API/releases/$release_id/assets?name=$name" >/dev/null
}

main() {
  [[ $# == 2 ]] || fatal 'usage: publish.sh RELEASE_DIRECTORY TAG'
  RELEASE_DIR=$1
  TAG=$2
  [[ $TAG =~ ^v([0-9]+\.[0-9]+\.[0-9]+)$ ]] ||
    fatal "invalid release tag: $TAG"
  VERSION=${BASH_REMATCH[1]}
  : "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"
  : "${GITHUB_SHA:?GITHUB_SHA is required}"
  : "${GITHUB_TOKEN:?GITHUB_TOKEN is required}"
  : "${IMMUTABLE_RELEASES_TOKEN:?IMMUTABLE_RELEASES_TOKEN is required}"
  API="/repos/$GITHUB_REPOSITORY"

  local expected=(
    install.sh
    SHA256SUMS
    "Fang_${VERSION}_amd64.deb"
    "fangd_${VERSION}-1_amd64.deb"
    "fang-${VERSION}-1.x86_64.rpm"
    "fangd-${VERSION}-1.x86_64.rpm"
  )
  local name
  local local_count
  [[ -d $RELEASE_DIR ]] || fatal "release directory does not exist: $RELEASE_DIR"
  for name in "${expected[@]}"; do
    [[ -f "$RELEASE_DIR/$name" ]] || fatal "missing local release asset: $name"
  done
  local_count=$(find "$RELEASE_DIR" -mindepth 1 -maxdepth 1 -type f -printf . | wc -c)
  [[ $local_count == 6 ]] || fatal "local release inventory contains $local_count files, expected 6"

  local immutable_enabled
  immutable_enabled=$(
    GH_TOKEN=$IMMUTABLE_RELEASES_TOKEN gh api \
      --method GET \
      -H 'Accept: application/vnd.github+json' \
      -H 'X-GitHub-Api-Version: 2026-03-10' \
      --jq '.enabled' \
      "$API/immutable-releases"
  ) || fatal 'could not read the repository immutable-release setting'
  [[ $immutable_enabled == true ]] ||
    fatal 'release immutability is not enabled for this repository'

  local existing_status
  PUBLICATION_TEMPORARY=$(mktemp -d)
  trap cleanup EXIT
  set +e
  GH_TOKEN=$GITHUB_TOKEN gh api \
    --method GET \
    --include \
    -H 'Accept: application/vnd.github+json' \
    -H 'X-GitHub-Api-Version: 2026-03-10' \
    "$API/releases/tags/$TAG" >"$PUBLICATION_TEMPORARY/existing" \
      2>"$PUBLICATION_TEMPORARY/existing.err"
  existing_status=$?
  set -e
  if [[ $existing_status == 0 ]]; then
    fatal "a release already exists for $TAG; it will not be changed"
  fi
  if ! grep -Eq '^HTTP/[^ ]+ 404([[:space:]]|$)' "$PUBLICATION_TEMPORARY/existing"; then
    cat "$PUBLICATION_TEMPORARY/existing.err" >&2
    fatal "could not safely establish that $TAG has no existing release"
  fi

  local release_id
  release_id=$(
    GH_TOKEN=$GITHUB_TOKEN gh api \
      --method POST \
      -H 'Accept: application/vnd.github+json' \
      -H 'X-GitHub-Api-Version: 2026-03-10' \
      -f "tag_name=$TAG" \
      -f "target_commitish=$GITHUB_SHA" \
      -f draft=true \
      -f prerelease=false \
      -f generate_release_notes=true \
      --jq '.id' \
      "$API/releases"
  ) || fatal 'could not create the release draft'
  [[ $release_id =~ ^[0-9]+$ ]] || fatal 'draft creation returned an invalid release ID'

  upload_asset "$release_id" install.sh
  upload_asset "$release_id" SHA256SUMS
  upload_asset "$release_id" "Fang_${VERSION}_amd64.deb"
  upload_asset "$release_id" "fangd_${VERSION}-1_amd64.deb"
  upload_asset "$release_id" "fang-${VERSION}-1.x86_64.rpm"
  upload_asset "$release_id" "fangd-${VERSION}-1.x86_64.rpm"

  GH_TOKEN=$GITHUB_TOKEN gh api \
    --method GET \
    -H 'Accept: application/vnd.github+json' \
    -H 'X-GitHub-Api-Version: 2026-03-10' \
    "$API/releases/$release_id" > "$PUBLICATION_TEMPORARY/draft.json"
  validate_release_json "$PUBLICATION_TEMPORARY/draft.json" draft

  GH_TOKEN=$GITHUB_TOKEN gh api \
    --method PATCH \
    -H 'Accept: application/vnd.github+json' \
    -H 'X-GitHub-Api-Version: 2026-03-10' \
    -f draft=false \
    -f prerelease=false \
    -f make_latest=true \
    "$API/releases/$release_id" > "$PUBLICATION_TEMPORARY/published-response.json"

  GH_TOKEN=$GITHUB_TOKEN gh api \
    --method GET \
    -H 'Accept: application/vnd.github+json' \
    -H 'X-GitHub-Api-Version: 2026-03-10' \
    "$API/releases/$release_id" > "$PUBLICATION_TEMPORARY/published.json"
  validate_release_json "$PUBLICATION_TEMPORARY/published.json" published

  GH_TOKEN=$GITHUB_TOKEN gh api \
    --method GET \
    -H 'Accept: application/vnd.github+json' \
    -H 'X-GitHub-Api-Version: 2026-03-10' \
    "$API/releases/latest" > "$PUBLICATION_TEMPORARY/latest.json"
  validate_release_json "$PUBLICATION_TEMPORARY/latest.json" published
  printf 'Published immutable Fang %s release with six verified assets.\n' "$TAG"
}

main "$@"
