#!/usr/bin/env bash
# Resolve a semantic version tag into the GitHub Release name.
# A manual run only builds the RPM and does not publish.
set -euo pipefail

cd "$(dirname "$0")/../.."

if [[ -z "${GITHUB_OUTPUT:-}" ]]; then
  echo "GITHUB_OUTPUT is not set" >&2
  exit 1
fi

pkg=$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' src-tauri/tauri.conf.json | head -n 1)
if [[ ! "$pkg" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "tauri.conf.json version must be MAJOR.MINOR.PATCH, got: ${pkg:-empty}" >&2
  exit 1
fi

if [[ "${GITHUB_REF:-}" != refs/tags/v* ]]; then
  {
    echo "publish=false"
    echo "tag="
    echo "prerelease=false"
  } >> "$GITHUB_OUTPUT"
  echo "Manual run builds the RPM only. Push tag v${pkg} for a release, or v${pkg}-rc.1 for a prerelease."
  exit 0
fi

tag="$GITHUB_REF_NAME"
if [[ ! "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "Tag ${tag} is not a semantic version. Example: v${pkg} or v${pkg}-rc.1" >&2
  exit 1
fi
base="${BASH_REMATCH[1]}"
pre="${BASH_REMATCH[2]:-}"
if [[ "$base" != "$pkg" ]]; then
  echo "Tag ${tag} has version ${base}, but the app version is ${pkg}. Update tauri.conf.json, Cargo.toml, and package.json before tagging." >&2
  exit 1
fi

prerelease=false
if [[ -n "$pre" ]]; then
  prerelease=true
fi

{
  echo "publish=true"
  echo "tag=${tag}"
  echo "prerelease=${prerelease}"
} >> "$GITHUB_OUTPUT"
echo "Publishing Typelet ${tag} (prerelease=${prerelease})"
