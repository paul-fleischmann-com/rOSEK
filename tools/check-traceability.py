#!/usr/bin/env python3
"""Checks requirement traceability between docs/req42/ and docs/autosar/.

Two things:
1. Link integrity: every `link:<path>#<ID>[...]` in docs/req42/ that points
   into docs/autosar/ must resolve to a file that actually exists and, if an
   anchor is given, to an `[[ID]]` anchor that actually exists in that file.
2. BCC1 coverage: for the AUTOSAR chapters relevant to the BCC1 scope (see
   BCC1_SCOPE below), which requirement IDs defined there are referenced by
   at least one req42 story, and which are not (yet).

Usage:
    python3 tools/check-traceability.py [--repo-root PATH]

Exit code is 1 if any broken link was found, 0 otherwise. Coverage gaps are
informational only and never affect the exit code — req42 only has to cover
"at least" BCC1, not every single requirement in scope.
"""
from __future__ import annotations

import argparse
import glob
import os
import re
import sys

# AsciiDoc link macro target: non-whitespace, ends right before the `[...]`
# text part. Deliberately stricter than "no brackets" so prose that merely
# contains the word "link:" (not followed immediately by a real target) isn't
# mistaken for a macro.
LINK_RE = re.compile(r"link:(\S+?)\[")
ANCHOR_RE = re.compile(r"^\[\[([A-Za-z0-9_]+)\]\]\s*$", re.MULTILINE)
ANCHOR_DEF_RE = re.compile(r"\[\[([A-Za-z0-9_]+)\]\]")

# Chapters that matter for the BCC1 scope (Phasen 0-3), as named in issue #28's
# acceptance criteria and #30's task description. Add a line here whenever a
# new AUTOSAR chapter becomes relevant to a covered phase.
BCC1_SCOPE = [
    "docs/autosar/sws_os/07-functional-specification/7.1-core-os.adoc",
    "docs/autosar/sws_os/07-functional-specification/7.6-os-application.adoc",
    "docs/autosar/sws_os/07-functional-specification/7.7-protection-facilities.adoc",
    "docs/autosar/rs_os/04-requirement-specification/4.2-real-time-operating-system.adoc",
    "docs/autosar/rs_os/04-requirement-specification/4.3-statically-defined-scheduling.adoc",
    "docs/autosar/rs_os/04-requirement-specification/4.5-protection-facilities.adoc",
    "docs/autosar/rs_os/04-requirement-specification/4.8-application-error-handling.adoc",
]


def read(path: str) -> str:
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def check_links(repo_root: str) -> tuple[int, int, list[str]]:
    """Returns (checked_count, broken_count, broken_details)."""
    checked = 0
    broken: list[str] = []
    req42_files = sorted(glob.glob(os.path.join(repo_root, "docs/req42/*.adoc")))
    for path in req42_files:
        text = read(path)
        rel_path = os.path.relpath(path, repo_root)
        for match in LINK_RE.finditer(text):
            target = match.group(1)
            if target.startswith("http") or "/autosar/" not in target:
                continue
            checked += 1
            file_part, _, anchor = target.partition("#")
            resolved = os.path.normpath(os.path.join(os.path.dirname(path), file_part))
            if not (os.path.isfile(resolved) or os.path.isdir(resolved)):
                broken.append(f"{rel_path}: {target} -> file not found ({os.path.relpath(resolved, repo_root)})")
                continue
            if anchor and os.path.isfile(resolved):
                content = read(resolved)
                if f"[[{anchor}]]" not in content:
                    broken.append(f"{rel_path}: {target} -> anchor '{anchor}' not found in {os.path.relpath(resolved, repo_root)}")
    return checked, len(broken), broken


def referenced_ids(repo_root: str) -> set[str]:
    """All requirement IDs referenced anywhere under docs/req42/, via link
    anchor fragment or plain `ID` mention (covers both `link:...#ID[...]`
    and any bare `SWS_Os_00001`-style mentions in prose)."""
    ids: set[str] = set()
    for path in glob.glob(os.path.join(repo_root, "docs/req42/*.adoc")):
        text = read(path)
        for match in LINK_RE.finditer(text):
            target = match.group(1)
            _, _, anchor = target.partition("#")
            if anchor:
                ids.add(anchor)
        ids.update(re.findall(r"\b(?:SWS|SRS|ECUC)_Os_[A-Za-z0-9_]+\b", text))
    return ids


def chapter_ids(path: str) -> list[str]:
    if not os.path.isfile(path):
        return []
    return ANCHOR_DEF_RE.findall(read(path))


def check_coverage(repo_root: str, referenced: set[str]) -> tuple[int, int, list[tuple[str, int, int, list[str]]]]:
    """Returns (total_ids, total_covered, per_chapter_details)."""
    details = []
    total_ids = 0
    total_covered = 0
    for rel_chapter in BCC1_SCOPE:
        abs_chapter = os.path.join(repo_root, rel_chapter)
        ids = chapter_ids(abs_chapter)
        covered = [i for i in ids if i in referenced]
        gap = [i for i in ids if i not in referenced]
        total_ids += len(ids)
        total_covered += len(covered)
        details.append((rel_chapter, len(ids), len(covered), gap))
    return total_ids, total_covered, details


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--repo-root", default=os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    args = parser.parse_args()
    repo_root = args.repo_root

    print("=== Traceability Check: docs/req42/ ===\n")

    print("-- Link integrity --")
    checked, broken_count, broken = check_links(repo_root)
    print(f"Checked {checked} link(s) into docs/autosar/.")
    if broken:
        print(f"{broken_count} BROKEN:")
        for line in broken:
            print(f"  - {line}")
    else:
        print("0 broken.")
    print()

    print("-- BCC1 Coverage --")
    referenced = referenced_ids(repo_root)
    total_ids, total_covered, details = check_coverage(repo_root, referenced)
    for rel_chapter, n_ids, n_covered, gap in details:
        pct = (100 * n_covered // n_ids) if n_ids else 0
        print(f"{rel_chapter}")
        print(f"  {n_covered}/{n_ids} requirements referenced ({pct}%)")
        if gap:
            print(f"  Not (yet) referenced: {', '.join(gap)}")
    print()
    total_pct = (100 * total_covered // total_ids) if total_ids else 0
    print(f"TOTAL BCC1 scope: {total_covered}/{total_ids} requirements referenced ({total_pct}%)")
    print("(Coverage is informational — req42 only has to cover BCC1 \"at least\", not exhaustively.)")

    if broken:
        print("\nFAILED: broken links found.", file=sys.stderr)
        return 1
    print("\nOK: no broken links.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
