#!/usr/bin/env python3
"""Antigravity PreToolUse guard enforcing branch protection and Conventional Commits."""

from __future__ import annotations

import json
import os
import re
import shlex
import subprocess
import sys
from pathlib import Path

PROTECTED_BRANCHES = {"main", "master"}
CONVENTIONAL_RE = re.compile(
    r"^(feat|fix|refactor|perf|style|test|docs|build|ci|chore|revert)"
    r"(\([a-zA-Z0-9._/-]+\))?(!)?: \S.*$"
)
WRITE_TOOLS = {
    "write_file",
    "edit_file",
    "write_to_file",
    "replace_file_content",
    "multi_replace_file_content",
}


def allow() -> None:
    print(json.dumps({"decision": "allow"}))
    sys.exit(0)


def deny(reason: str) -> None:
    print(json.dumps({"decision": "deny", "reason": reason}))
    sys.exit(0)


def run_git(cwd: str, *args: str) -> tuple[int, str]:
    try:
        res = subprocess.run(
            ["git", "-C", cwd, *args],
            capture_output=True,
            text=True,
            check=False,
        )
        return res.returncode, res.stdout.strip()
    except OSError:
        return 1, ""


def existing_dir(path_str: str, fallback: str) -> str:
    p = Path(path_str)
    if not p.is_absolute():
        p = Path(fallback) / p
    cur = p if p.is_dir() else p.parent
    while not cur.exists() and cur != cur.parent:
        cur = cur.parent
    return str(cur if cur.exists() else Path(fallback))


def is_inside_repo(target_path: str, cwd: str) -> tuple[bool, str | None]:
    target_dir = existing_dir(target_path, cwd)
    rc, root = run_git(target_dir, "rev-parse", "--show-toplevel")
    if rc != 0 or not root:
        return False, None
    rc_b, branch = run_git(target_dir, "symbolic-ref", "--quiet", "--short", "HEAD")
    return True, (branch if rc_b == 0 else None)


def current_branch(cwd: str) -> str | None:
    rc, branch = run_git(cwd, "symbolic-ref", "--quiet", "--short", "HEAD")
    return branch if rc == 0 else None


def split_shell_segments(command: str) -> list[str]:
    """Split shell command on unquoted ;, &&, ||, |, and newlines."""
    segments: list[str] = []
    buf: list[str] = []
    in_single = False
    in_double = False
    escape = False
    i = 0
    n = len(command)
    while i < n:
        ch = command[i]
        if escape:
            buf.append(ch)
            escape = False
            i += 1
            continue
        if ch == "\\" and not in_single:
            buf.append(ch)
            escape = True
            i += 1
            continue
        if ch == "'" and not in_double:
            in_single = not in_single
            buf.append(ch)
            i += 1
            continue
        if ch == '"' and not in_single:
            in_double = not in_double
            buf.append(ch)
            i += 1
            continue
        if not in_single and not in_double:
            if command[i : i + 2] in ("&&", "||"):
                seg = "".join(buf).strip()
                if seg:
                    segments.append(seg)
                buf.clear()
                i += 2
                continue
            if ch in (";", "|", "\n"):
                seg = "".join(buf).strip()
                if seg:
                    segments.append(seg)
                buf.clear()
                i += 1
                continue
        buf.append(ch)
        i += 1
    seg = "".join(buf).strip()
    if seg:
        segments.append(seg)
    return segments


def parse_git_invocation(tokens: list[str]) -> tuple[int, str, list[str]] | None:
    """Return (git_idx, subcommand, subcommand_args) if tokens invoke git."""
    idx = 0
    # Skip leading env assignments or 'env' wrapper.
    while idx < len(tokens):
        tok = tokens[idx]
        if tok == "env" or ("=" in tok and not tok.startswith("-")):
            idx += 1
            continue
        break
    if idx >= len(tokens):
        return None
    if os.path.basename(tokens[idx]) != "git":
        return None

    i = idx + 1
    takes_value = {"-C", "-c", "--git-dir", "--work-tree", "--namespace", "--exec-path"}
    while i < len(tokens):
        tok = tokens[i]
        if tok in takes_value:
            i += 2
            continue
        if tok.startswith("-"):
            i += 1
            continue
        return idx, tok, tokens[i + 1 :]
    return None


def validate_commit_args(args: list[str]) -> str | None:
    if "--no-verify" in args or "-n" in args:
        return "Blocked: '--no-verify' / '-n' on git commit is forbidden."

    messages: list[str] = []
    i = 0
    while i < len(args):
        tok = args[i]
        if tok in ("-m", "--message") and i + 1 < len(args):
            messages.append(args[i + 1])
            i += 2
            continue
        if tok.startswith("--message="):
            messages.append(tok.split("=", 1)[1])
        elif tok.startswith("-m") and len(tok) > 2:
            messages.append(tok[2:])
        i += 1

    if messages:
        subject = messages[0].splitlines()[0].strip()
        if not subject.startswith("Merge ") and not CONVENTIONAL_RE.match(subject):
            return (
                f"Blocked: commit message '{subject}' does not follow Conventional Commits "
                "(<type>[optional scope]: <description>, e.g. 'feat(protocol): add parser')."
            )
    return None


def validate_push_args(args: list[str], active_branch: str | None) -> str | None:
    if "--no-verify" in args:
        return "Blocked: '--no-verify' on git push is forbidden."

    positional: list[str] = []
    skip_next = False
    value_flags = {"-o", "--push-option", "--receive-pack", "--exec", "--repo"}
    for tok in args:
        if skip_next:
            skip_next = False
            continue
        if tok in value_flags:
            skip_next = True
            continue
        if tok.startswith("-"):
            continue
        positional.append(tok)

    # positional[0] is remote (if present), positional[1:] are refspecs.
    refspecs = positional[1:]
    if not refspecs:
        if active_branch in PROTECTED_BRANCHES:
            return f"Blocked: pushing while on protected branch '{active_branch}' is forbidden."
        return None

    for spec in refspecs:
        dest = spec.split(":", 1)[1] if ":" in spec else spec
        dest_clean = dest.removeprefix("refs/heads/")
        if dest_clean in PROTECTED_BRANCHES:
            return f"Blocked: pushing to protected branch '{dest_clean}' is forbidden."
        if dest in ("HEAD", "") and active_branch in PROTECTED_BRANCHES:
            return f"Blocked: pushing HEAD from protected branch '{active_branch}' is forbidden."
    return None


def check_run_command(cmd: str, cwd: str) -> None:
    active_branch = current_branch(cwd)
    segments = split_shell_segments(cmd)

    for seg in segments:
        try:
            tokens = shlex.split(seg)
        except ValueError:
            tokens = seg.split()
        if not tokens:
            continue

        parsed = parse_git_invocation(tokens)
        if not parsed:
            continue

        _, subcmd, subargs = parsed

        # Track in-command branch creation/switching so `git switch -c x && git commit` works.
        if subcmd == "switch":
            if "-c" in subargs or "-C" in subargs:
                active_branch = "feature-branch"
            else:
                pos = [a for a in subargs if not a.startswith("-")]
                if pos:
                    active_branch = pos[-1]
            continue
        if subcmd == "checkout":
            if "-b" in subargs or "-B" in subargs:
                active_branch = "feature-branch"
            else:
                pos = [a for a in subargs if not a.startswith("-")]
                if pos and pos[0] not in ("--", "."):
                    active_branch = pos[0]
            continue

        if subcmd == "commit":
            if active_branch in PROTECTED_BRANCHES:
                deny(
                    f"Blocked: cannot commit on '{active_branch}'. "
                    "Create a working branch first (e.g. git switch -c feat/<name>)."
                )
            err = validate_commit_args(subargs)
            if err:
                deny(err)

        elif subcmd == "push":
            err = validate_push_args(subargs, active_branch)
            if err:
                deny(err)

        elif subcmd in {"merge", "rebase", "cherry-pick", "am", "apply"}:
            if active_branch in PROTECTED_BRANCHES:
                deny(
                    f"Blocked: cannot run 'git {subcmd}' on protected branch '{active_branch}'."
                )

    allow()


def main() -> None:
    try:
        data = json.load(sys.stdin)
    except Exception:
        allow()
        return

    cwd = data.get("cwd") or (data.get("workspacePaths") or ["."])[0]
    tool_call = data.get("toolCall") or {}
    tool_name = tool_call.get("name", "")
    args = tool_call.get("args") or {}

    if tool_name in WRITE_TOOLS:
        target = args.get("Path") or args.get("TargetFile") or args.get("path") or ""
        if target:
            inside, branch = is_inside_repo(target, cwd)
            if inside and branch in PROTECTED_BRANCHES:
                deny(
                    f"Blocked: repository is on '{branch}'. Agents must never write to "
                    "main/master. Run 'git switch -c <type>/<short-name>' first."
                )
        allow()

    if tool_name == "run_command":
        cmd = args.get("CommandLine") or ""
        check_run_command(cmd, cwd)

    allow()


if __name__ == "__main__":
    main()
