#!/bin/sh
set -eu

payload=$(cat || true)

cwd="."
if command -v python3 >/dev/null 2>&1; then
  cwd=$(printf '%s' "$payload" | python3 -c '
import json, sys
try:
    data = json.load(sys.stdin)
    print(data.get("cwd") or (data.get("workspacePaths") or ["."])[0])
except Exception:
    print(".")
' 2>/dev/null || echo ".")
elif command -v jq >/dev/null 2>&1; then
  cwd=$(printf '%s' "$payload" | jq -r '.cwd // .workspacePaths[0] // "."' 2>/dev/null || echo ".")
fi

repo_root=$(git -C "$cwd" rev-parse --show-toplevel 2>/dev/null || true)
if [ -n "$repo_root" ] && [ -d "$repo_root/.githooks" ]; then
  current_hooks=$(git -C "$cwd" config --get core.hooksPath 2>/dev/null || true)
  if [ -z "$current_hooks" ]; then
    git -C "$cwd" config core.hooksPath .githooks 2>/dev/null || true
  fi
fi

branch=$(git -C "$cwd" symbolic-ref --quiet --short HEAD 2>/dev/null || echo "detached")

if [ "$branch" = "main" ] || [ "$branch" = "master" ]; then
  msg="WORKSPACE RULE: Current git branch is '$branch'. NEVER write files or commit/push on main/master. Before making any edits, create and switch to a feature branch (e.g., git switch -c feat/<short-name>). Use small, incremental Conventional Commits—never one giant final commit."
else
  msg="WORKSPACE RULE: Active branch is '$branch'. Keep commits small and incremental using Conventional Commits (<type>(scope): <description>); never write, commit, or push to main/master."
fi

if command -v python3 >/dev/null 2>&1; then
  MSG="$msg" python3 -c '
import json, os
print(json.dumps({
    "injectSteps": [
        {"systemMessage": {"systemMessage": os.environ["MSG"]}}
    ]
}))
'
else
  printf '{"injectSteps":[{"systemMessage":{"systemMessage":"%s"}}]}\n' "$(printf '%s' "$msg" | sed 's/"/\\"/g')"
fi
