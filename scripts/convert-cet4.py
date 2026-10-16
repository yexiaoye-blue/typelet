#!/usr/bin/env python3
"""Convert kajweb/dict CET4_3 JSON-lines data to Typelet's flat word records."""
import argparse
import json
from pathlib import Path

EXPECTED_COUNT = 2607


def convert(source: Path, destination: Path) -> None:
    records = []
    names = set()
    with source.open(encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, 1):
            try:
                record = json.loads(line)
            except json.JSONDecodeError as exc:
                raise SystemExit(f"invalid JSON at line {line_number}: {exc}") from exc
            word = record.get("content", {}).get("word", {})
            content = word.get("content", {})
            name = (word.get("wordHead") or record.get("headWord") or "").strip()
            if not name:
                raise SystemExit(f"missing name at line {line_number}")
            if name in names:
                raise SystemExit(f"duplicate name {name!r} at line {line_number}")
            names.add(name)
            translations = [
                item.get("tranCn", "").strip()
                for item in content.get("trans", [])
                if item.get("tranCn", "").strip()
            ]
            records.append({
                "name": name,
                "trans": translations,
                "usphone": content.get("usphone", "") or "",
                "ukphone": content.get("ukphone", "") or "",
            })
    if len(records) != EXPECTED_COUNT:
        raise SystemExit(f"expected {EXPECTED_COUNT} records, got {len(records)}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(records, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"wrote {len(records)} records to {destination}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    convert(args.source, args.destination)
