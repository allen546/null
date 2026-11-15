#!/usr/bin/env -S uv run --no-project python
"""Lists (family, block) for every block the fork predicts, from azalea-block/src/place_families.rs.

Usage: uv run --no-project harness/vanilla-place/families_from_fork.py <place_families.rs> > harness/vanilla-place/families.tsv
"""
import re
import sys

text = open(sys.argv[1]).read()
start = text.index("pub fn place_family")
body = text[start:text.index("\n}\n", start)]
print("# family\tblock (CamelCase as in azalea-block place_families.rs)")
for m in re.finditer(r"((?:\s*BlockKind::\w+\s*\|?)+)=>\s*PlaceFamily::(\w+)", body):
    for kind in re.findall(r"BlockKind::(\w+)", m.group(1)):
        print(f"{m.group(2)}\t{kind}")
