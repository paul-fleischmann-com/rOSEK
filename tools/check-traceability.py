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
    python3 tools/check-traceability.py [--repo-root PATH] [--markdown]

--markdown renders the same report as GitHub-flavored Markdown (used for the
GitHub Actions job summary) instead of the default plain-text form.

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


def gather(repo_root: str):
    checked, broken_count, broken = check_links(repo_root)
    referenced = referenced_ids(repo_root)
    total_ids, total_covered, details = check_coverage(repo_root, referenced)
    return checked, broken, total_ids, total_covered, details


def render_text(checked, broken, total_ids, total_covered, details) -> str:
    out = ["=== Traceability Check: docs/req42/ ===\n"]
    out.append("-- Link integrity --")
    out.append(f"Checked {checked} link(s) into docs/autosar/.")
    if broken:
        out.append(f"{len(broken)} BROKEN:")
        out.extend(f"  - {line}" for line in broken)
    else:
        out.append("0 broken.")
    out.append("")

    out.append("-- BCC1 Coverage --")
    for rel_chapter, n_ids, n_covered, gap in details:
        pct = (100 * n_covered // n_ids) if n_ids else 0
        out.append(rel_chapter)
        out.append(f"  {n_covered}/{n_ids} requirements referenced ({pct}%)")
        if gap:
            out.append(f"  Not (yet) referenced: {', '.join(gap)}")
    out.append("")
    total_pct = (100 * total_covered // total_ids) if total_ids else 0
    out.append(f"TOTAL BCC1 scope: {total_covered}/{total_ids} requirements referenced ({total_pct}%)")
    out.append('(Coverage is informational — req42 only has to cover BCC1 "at least", not exhaustively.)')

    out.append("")
    out.append("FAILED: broken links found." if broken else "OK: no broken links.")
    return "\n".join(out)


def render_markdown(checked, broken, total_ids, total_covered, details) -> str:
    out = ["## Requirements Traceability Report", ""]
    out.append(f"_docs/req42/ ↔ docs/autosar/ — generated by `tools/check-traceability.py`_")
    out.append("")

    status = "❌ FAILED" if broken else "✅ OK"
    out.append(f"### Link integrity: {status}")
    out.append("")
    out.append(f"Checked **{checked}** link(s) into `docs/autosar/`, **{len(broken)}** broken.")
    if broken:
        out.append("")
        out.append("| Source | Broken link |")
        out.append("|---|---|")
        for line in broken:
            src, _, rest = line.partition(": ")
            out.append(f"| `{src}` | {rest} |")
    out.append("")

    total_pct = (100 * total_covered // total_ids) if total_ids else 0
    out.append(f"### BCC1 Coverage: {total_covered}/{total_ids} ({total_pct}%)")
    out.append("")
    out.append("_Informational only — req42 only has to cover BCC1 \"at least\", not exhaustively._")
    out.append("")
    out.append("| Chapter | Referenced | Coverage |")
    out.append("|---|---|---|")
    for rel_chapter, n_ids, n_covered, gap in details:
        pct = (100 * n_covered // n_ids) if n_ids else 0
        out.append(f"| `{rel_chapter}` | {n_covered}/{n_ids} | {pct}% |")
    out.append("")

    for rel_chapter, n_ids, n_covered, gap in details:
        if not gap:
            continue
        out.append(f"<details><summary>Not (yet) referenced in <code>{rel_chapter}</code> ({len(gap)})</summary>")
        out.append("")
        out.append(", ".join(f"`{i}`" for i in gap))
        out.append("")
        out.append("</details>")
        out.append("")

    return "\n".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--repo-root", default=os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    parser.add_argument("--markdown", action="store_true", help="Render as GitHub-flavored Markdown instead of plain text")
    args = parser.parse_args()

    checked, broken, total_ids, total_covered, details = gather(args.repo_root)

    if args.markdown:
        print(render_markdown(checked, broken, total_ids, total_covered, details))
    else:
        print(render_text(checked, broken, total_ids, total_covered, details))

    if broken:
        if not args.markdown:
            print("", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
