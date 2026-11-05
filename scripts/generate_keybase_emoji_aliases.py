#!/usr/bin/env python3
"""Regenerate standard emoji aliases from Keybase's generated Go emojiCodeMap."""
import argparse
import ast
import json
from pathlib import Path
import re

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path, help="Keybase go/chat/storage/emoji_codemap.go")
args = parser.parse_args()
source = args.source.read_text().split("var emojiCodeMap = map[string]string{", 1)[1].split("\n}", 1)[0]
entries = re.findall(r'^\s*("(?:[^"\\]|\\.)*"):\s*("(?:[^"\\]|\\.)*"),', source, re.MULTILINE)
aliases = {ast.literal_eval(alias).strip(":"): ast.literal_eval(value) for alias, value in entries}
if not aliases or aliases.get("large_green_circle") != "🟢":
    raise SystemExit("Unrecognized Keybase emoji table")
output = Path(__file__).resolve().parents[1] / "assets/emoji/keybase-shortcodes.json"
output.write_text(json.dumps(aliases, ensure_ascii=False, indent=2, sort_keys=True) + "\n")
print(f"Generated {len(aliases)} aliases in {output}")
