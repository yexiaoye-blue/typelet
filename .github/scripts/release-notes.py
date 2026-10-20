#!/usr/bin/env python3
"""Build the GitHub Release description for one Typelet build.

Categories and skipped commit types live in .github/release-notes.json.
A commit is classified from its subject prefix, for example "feat:" or
"fix(audio):". To place one commit in more than one category, add sections
to the commit body:

    新增:
    - 设置页显示当前版本

    修复:
    - 按键音不再卡住后连串播放
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / ".github" / "release-notes.json"
SUBJECT = re.compile(r"^(?P<type>[A-Za-z]+)(?:\([^)]+\))?!?:\s*(?P<description>.+)$")

def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True)

def previous_tag(head: str, current_tag: str) -> str | None:
    tags = git("tag", "--merged", head, "--sort=-creatordate").split()
    for tag in tags:
        if tag != current_tag:
            return tag
    return None

def commit_entries(previous: str | None, head: str) -> list[tuple[str, str]]:
    revision = f"{previous}..{head}" if previous else head
    output = git("log", "--no-merges", "--reverse", "--format=%s%x1f%b%x1e", revision)
    entries: list[tuple[str, str]] = []
    for record in output.split("\x1e"):
        record = record.strip()
        if not record:
            continue
        subject, _, body = record.partition("\x1f")
        entries.append((subject.strip(), body.strip()))
    return entries

def normalize_title(line: str, titles: set[str]) -> str | None:
    header = line.strip().lstrip("#").strip().rstrip(":：").strip()
    return header if header in titles else None

def bullets_from_body(body: str, titles: set[str]) -> dict[str, list[str]] | None:
    sections: dict[str, list[str]] = {}
    current: str | None = None
    found = False
    for raw in body.splitlines():
        line = raw.strip()
        title = normalize_title(line, titles)
        if title and (line.endswith(":") or line.endswith("：") or line.startswith("#")):
            current = title
            found = True
            sections.setdefault(current, [])
            continue
        if current and (line.startswith("- ") or line.startswith("* ")):
            text = line[2:].strip()
            if text:
                sections[current].append(text)
    return sections if found else None

def render(categories: list[dict[str, object]], grouped: dict[str, list[str]], previous: str | None, tag: str) -> str:
    parts: list[str] = []
    for category in categories:
        title = str(category["title"])
        items = grouped.get(title, [])
        if not items:
            continue
        parts.append(f"## {title}")
        parts.append("")
        parts.extend(f"- {item}" for item in items)
        parts.append("")
    if not parts:
        parts.extend(["这次构建没有新的功能或修复说明。", ""])
    repository = os.environ.get("GITHUB_REPOSITORY", "")
    if previous and tag and repository:
        compare = f"https://github.com/{repository}/compare/{previous}...{tag}"
        parts.extend(["---", "", f"**完整变更**：[{previous}...{tag}]({compare})", ""])
    return "\n".join(parts).rstrip() + "\n"

def main() -> int:
    config = json.loads(CONFIG.read_text(encoding="utf-8"))
    categories = list(config["categories"])
    skip = {item.casefold() for item in config.get("skipTypes", [])}
    titles = [str(category["title"]) for category in categories]
    title_set = set(titles)
    type_to_title: dict[str, str] = {}
    fallback = ""
    for category in categories:
        title = str(category["title"])
        for name in category.get("types", []):
            kind = str(name)
            if kind == "*":
                fallback = title
            else:
                type_to_title[kind.casefold()] = title

    head = os.environ.get("GITHUB_SHA") or git("rev-parse", "HEAD").strip()
    tag = os.environ.get("RELEASE_TAG", "")
    previous = previous_tag(head, tag)
    grouped = {title: [] for title in titles}
    for subject, body in commit_entries(previous, head):
        sections = bullets_from_body(body, title_set)
        if sections is not None:
            for title, items in sections.items():
                grouped[title].extend(items)
            continue
        match = SUBJECT.match(subject)
        if match:
            kind = match.group("type").casefold()
            description = match.group("description").strip()
            if kind in skip:
                continue
            title = type_to_title.get(kind, fallback)
        else:
            description = subject.strip()
            title = fallback
        if title and description:
            grouped[title].append(description)
    sys.stdout.write(render(categories, grouped, previous, tag))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())

