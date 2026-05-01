#!/usr/bin/env python3
"""Check local links in tracked Markdown files."""

from __future__ import annotations

import argparse
import os
import posixpath
import re
import subprocess
import sys
import unicodedata
from dataclasses import dataclass
from pathlib import PurePosixPath
from urllib.parse import unquote


SKIP_PARTS = {"target", ".git", "node_modules", "vendor", "third_party"}
SCHEME_RE = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
REFERENCE_DEF_RE = re.compile(r"^[ ]{0,3}\[([^\]]+)\]:[ \t]*(.*)$")
REFERENCE_LINK_RE = re.compile(r"!?\[([^\]\n]+)\]\[([^\]\n]*)\]")
HTML_LINK_RE = re.compile(r"""\b(?:href|src)=["']([^"']+)["']""", re.IGNORECASE)
HEADING_RE = re.compile(r"^[ ]{0,3}(#{1,6})[ \t]+(.+?)[ \t]*#*[ \t]*$")
HTML_ID_RE = re.compile(r"""\b(?:id|name)=["']([^"']+)["']""", re.IGNORECASE)
EXPLICIT_ID_RE = re.compile(r"\s+\{#([A-Za-z0-9_.:-]+)\}\s*$")
TAG_RE = re.compile(r"<[^>]+>")
PUNCT_RE = re.compile(r"[^\w\- ]+", re.UNICODE)
LINE_FRAGMENT_RE = re.compile(r"^L\d+(?:-L\d+)?$")


@dataclass(frozen=True)
class Link:
    source: str
    line: int
    dest: str


def run_git(repo: str, *args: str) -> str:
    proc = subprocess.run(
        ("git", "-C", repo, *args),
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return proc.stdout


def repo_root() -> str:
    return run_git(".", "rev-parse", "--show-toplevel").strip()


def tracked_files(repo: str) -> set[str]:
    out = run_git(repo, "ls-files")
    return {line for line in out.splitlines() if line}


def dirs_for(files: set[str]) -> set[str]:
    dirs: set[str] = set()
    for path in files:
        parts = PurePosixPath(path).parts
        for i in range(1, len(parts)):
            dirs.add("/".join(parts[:i]))
    return dirs


def markdown_files(files: set[str]) -> list[str]:
    return sorted(
        path
        for path in files
        if path.lower().endswith(".md")
        and not any(part in SKIP_PARTS for part in PurePosixPath(path).parts)
    )


def read_text(repo: str, path: str) -> str:
    with open(os.path.join(repo, path), encoding="utf-8", errors="replace") as f:
        return f.read()


def iter_non_code_lines(text: str):
    fence: str | None = None
    for lineno, line in enumerate(text.splitlines(), 1):
        stripped = line.lstrip(" ")
        marker = stripped[:3]
        indent = len(line) - len(stripped)
        if indent <= 3 and marker in ("```", "~~~"):
            if fence is None:
                fence = marker
            elif marker == fence:
                fence = None
            continue
        if fence is None:
            yield lineno, line


def parse_destination(raw: str) -> str:
    value = raw.strip()
    if not value:
        return ""
    if value.startswith("<"):
        end = value.find(">")
        return value[1:end].strip() if end >= 0 else value[1:].strip()

    escaped = False
    for i, char in enumerate(value):
        if escaped:
            escaped = False
        elif char == "\\":
            escaped = True
        elif char.isspace():
            return value[:i].strip()
    return value


def normalize_label(label: str) -> str:
    return " ".join(label.strip().casefold().split())


def collect_reference_defs(text: str) -> dict[str, str]:
    refs: dict[str, str] = {}
    for _lineno, line in iter_non_code_lines(text):
        match = REFERENCE_DEF_RE.match(line)
        if not match:
            continue
        label = normalize_label(match.group(1))
        dest = parse_destination(match.group(2))
        if label and dest:
            refs[label] = dest
    return refs


def inline_links(source: str, lineno: int, line: str) -> list[Link]:
    links: list[Link] = []
    i = 0
    while True:
        start = line.find("](", i)
        if start < 0:
            return links
        if start > 0 and line[start - 1] == "\\":
            i = start + 2
            continue

        pos = start + 2
        depth = 1
        escaped = False
        while pos < len(line):
            char = line[pos]
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
                if depth == 0:
                    dest = parse_destination(line[start + 2 : pos])
                    if dest:
                        links.append(Link(source, lineno, dest))
                    break
            pos += 1
        i = pos + 1


def extract_links(source: str, text: str) -> list[Link]:
    refs = collect_reference_defs(text)
    links: list[Link] = []
    for lineno, line in iter_non_code_lines(text):
        if REFERENCE_DEF_RE.match(line):
            continue
        links.extend(inline_links(source, lineno, line))
        for match in REFERENCE_LINK_RE.finditer(line):
            label = match.group(2) or match.group(1)
            dest = refs.get(normalize_label(label))
            links.append(Link(source, lineno, dest or f"[missing-ref]:{label}"))
        for match in HTML_LINK_RE.finditer(line):
            links.append(Link(source, lineno, match.group(1).strip()))
    return links


def is_external(dest: str) -> bool:
    stripped = dest.strip()
    return stripped.startswith("//") or stripped.startswith("/") or bool(SCHEME_RE.match(stripped))


def split_dest(dest: str) -> tuple[str, str | None]:
    path = dest
    fragment: str | None = None
    if "#" in path:
        path, fragment = path.split("#", 1)
    if "?" in path:
        path = path.split("?", 1)[0]
    return unquote(path), unquote(fragment or "") if fragment is not None else None


def normalize_target(source: str, raw_path: str) -> str:
    if raw_path in ("", "."):
        return source
    base = posixpath.dirname(source)
    return posixpath.normpath(posixpath.join(base, raw_path))


def slugify(heading: str) -> str:
    heading = EXPLICIT_ID_RE.sub("", heading)
    heading = TAG_RE.sub("", heading)
    heading = unicodedata.normalize("NFKD", heading.strip().strip("#").strip())
    heading = heading.casefold()
    heading = PUNCT_RE.sub("", heading)
    heading = re.sub(r"\s+", "-", heading.strip())
    return re.sub(r"-+", "-", heading)


def anchors_for(text: str) -> set[str]:
    anchors: set[str] = set()
    seen: dict[str, int] = {}
    for _lineno, line in iter_non_code_lines(text):
        for match in HTML_ID_RE.finditer(line):
            anchors.add(match.group(1))

        match = HEADING_RE.match(line)
        if not match:
            continue
        heading = match.group(2)
        explicit = EXPLICIT_ID_RE.search(heading)
        if explicit:
            anchors.add(explicit.group(1))

        base = slugify(heading)
        if not base:
            continue
        count = seen.get(base, 0)
        seen[base] = count + 1
        anchors.add(base if count == 0 else f"{base}-{count}")
    return anchors


class Checker:
    def __init__(self, repo: str):
        self.repo = repo
        self.files = tracked_files(repo)
        self.dirs = dirs_for(self.files)
        self.anchor_cache: dict[str, set[str]] = {}

    def markdown_target(self, path: str) -> str | None:
        if path in self.files and path.lower().endswith(".md"):
            return path
        readme = posixpath.join(path, "README.md")
        if readme in self.files:
            return readme
        return None

    def target_exists(self, path: str) -> bool:
        return path in self.files or path in self.dirs

    def has_anchor(self, path: str, fragment: str | None) -> bool:
        if fragment is None or fragment == "":
            return True
        fragment = fragment.removeprefix("user-content-")
        md_path = self.markdown_target(path)
        if md_path is None:
            return bool(LINE_FRAGMENT_RE.match(fragment))
        if md_path not in self.anchor_cache:
            self.anchor_cache[md_path] = anchors_for(read_text(self.repo, md_path))
        return fragment in self.anchor_cache[md_path]

    def validate(self, link: Link) -> str | None:
        if link.dest.startswith("[missing-ref]:"):
            return f"missing reference definition for [{link.dest.split(':', 1)[1]}]"
        if is_external(link.dest):
            return None

        raw_path, fragment = split_dest(link.dest)
        target = normalize_target(link.source, raw_path)
        if target == ".." or target.startswith("../"):
            return None
        if not self.target_exists(target):
            return f"target does not exist: {link.dest}"
        if not self.has_anchor(target, fragment):
            return f"anchor does not exist: {link.dest}"
        return None


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--all",
        action="store_true",
        help="accepted for consistency; all tracked Markdown files are checked",
    )
    parser.add_argument(
        "--repo-root",
        default=None,
        help="repository root to check (defaults to git rev-parse --show-toplevel)",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo = os.path.abspath(args.repo_root or repo_root())
    checker = Checker(repo)
    errors: list[tuple[Link, str]] = []

    for path in markdown_files(checker.files):
        for link in extract_links(path, read_text(repo, path)):
            message = checker.validate(link)
            if message:
                errors.append((link, message))

    if errors:
        print("ERROR: broken internal Markdown link(s):", file=sys.stderr)
        for link, message in errors:
            print(f"  {link.source}:{link.line}: {message} ({link.dest})", file=sys.stderr)
        print("\nExternal URLs and site-root paths are not checked.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
