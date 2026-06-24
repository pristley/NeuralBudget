#!/usr/bin/env python3

"""Generate release notes and backfill CHANGELOG.md for a tagged release."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def classify_commit(message: str) -> tuple[str, str]:
    lower = message.lower()
    if lower.startswith("feat"):
        return "Added", message
    if lower.startswith(("fix", "perf")):
        return "Fixed", message
    if lower.startswith(("docs", "refactor", "chore", "ci", "build", "test", "style")):
        return "Changed", message
    return "Changed", message


def build_categorized_sections(commits: list[str]) -> list[str]:
    sections: dict[str, list[str]] = {"Added": [], "Changed": [], "Fixed": []}
    for commit in commits:
        section, normalized = classify_commit(commit)
        sections[section].append(normalized)

    lines: list[str] = []
    for heading in ("Added", "Changed", "Fixed"):
        items = sections[heading]
        if not items:
            continue
        lines.extend([f"### {heading}", ""])
        lines.extend(items)
        lines.append("")

    return lines


def run_git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def build_release_section(tag_name: str) -> str:
    version = tag_name.lstrip("v")
    tag_commit = run_git("rev-parse", f"{tag_name}^{{}}")

    previous_tag = next(
        (tag for tag in run_git("tag", "--sort=-creatordate").splitlines() if tag and tag != tag_name),
        None,
    )

    if previous_tag:
        commit_range = f"{previous_tag}..{tag_commit}"
    else:
        commit_range = tag_commit

    raw_commits = run_git(
        "log",
        "--no-merges",
        "--pretty=format:- %s",
        commit_range,
    )
    commits = [line.strip() for line in raw_commits.splitlines() if line.strip()]
    if not commits:
        commits = ["- No code changes."]

    date = run_git("show", "-s", "--format=%cs", tag_commit)

    lines = [f"## [{version}] - {date}", ""]
    lines.extend(build_categorized_sections(commits))
    return "\n".join(lines)


def update_changelog(tag_name: str, release_notes_path: Path | None) -> None:
    changelog = Path("CHANGELOG.md")
    existing = changelog.read_text()
    marker = "All notable changes to this project will be documented in this file."
    if marker not in existing:
        raise RuntimeError("CHANGELOG.md does not match the expected header")

    release_section = build_release_section(tag_name)

    version_heading = release_section.splitlines()[0]
    if version_heading in existing:
        if release_notes_path is not None:
            release_notes_path.write_text(release_section)
        return

    header, remainder = existing.split(marker, 1)
    prefix = f"{header}{marker}\n\nRelease entries are maintained automatically by the CD workflow on tagged releases.\n\n"
    changelog.write_text(f"{prefix}{release_section}{remainder.lstrip()}")

    if release_notes_path is not None:
        release_notes_path.write_text(release_section)


def main() -> int:
    if len(sys.argv) not in {2, 3}:
        print("usage: update_changelog.py <tag-name> [release-notes-path]", file=sys.stderr)
        return 1

    tag_name = sys.argv[1]
    release_notes_path = Path(sys.argv[2]) if len(sys.argv) == 3 else None
    update_changelog(tag_name, release_notes_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())