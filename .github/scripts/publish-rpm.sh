#!/usr/bin/env bash
# 版本标签发布正式版；其余触发更新 test 预发布版本。
set -euo pipefail

cd "$(dirname "$0")/../.."

shopt -s nullglob
rpm_files=(src-tauri/target/release/bundle/rpm/*.rpm)
if [[ ${#rpm_files[@]} -ne 1 ]]; then
  echo "期望恰好一个 RPM，实际找到 ${#rpm_files[@]} 个" >&2
  exit 1
fi
rpm_file="${rpm_files[0]}"

notes=$(printf 'Fedora 44 构建。\n\n提交：%s\n来源：%s\n' "$GITHUB_SHA" "$GITHUB_REF")

if [[ "$GITHUB_REF" == refs/tags/v* ]]; then
  tag="$GITHUB_REF_NAME"
  if gh release view "$tag" >/dev/null 2>&1; then
    gh release upload "$tag" "$rpm_file" --clobber
  else
    gh release create "$tag" "$rpm_file" \
      --title "Typelet ${tag}" \
      --notes "$notes"
  fi
  exit 0
fi

gh release delete test --yes --cleanup-tag || true
gh api --method DELETE "repos/${GITHUB_REPOSITORY}/git/refs/tags/test" >/dev/null 2>&1 || true
gh release create test "$rpm_file" \
  --prerelease \
  --target "$GITHUB_SHA" \
  --title "Typelet test" \
  --notes "$notes"
