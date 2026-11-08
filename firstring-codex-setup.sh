#!/usr/bin/env bash
# FirstRing: Project-Local Skill Installer
# Copies pinned skill files only. Does not run upstream code or install runtimes.
# Usage: bash firstring-codex-setup.sh [PROJECT_DIRECTORY | --dry-run | --help]
set -euo pipefail

die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

# name|cache-name|repository|commit|skill-directory|license-file
MANIFEST='rust-async-patterns|rust-agents|https://github.com/wshobson/agents.git|4bde3168b43c0655c4d00ea2726e631719908a6d|plugins/systems-programming/skills/rust-async-patterns|LICENSE
twilio-voice-twiml|twilio-ai|https://github.com/twilio/ai.git|803790a281f0ef11d2928ffb41bc8422237fb434|skills/twilio/twilio-voice-twiml|LICENSE
twilio-webhook-architecture|twilio-ai|https://github.com/twilio/ai.git|803790a281f0ef11d2928ffb41bc8422237fb434|skills/twilio/twilio-webhook-architecture|LICENSE
twilio-security-hardening|twilio-ai|https://github.com/twilio/ai.git|803790a281f0ef11d2928ffb41bc8422237fb434|skills/twilio/twilio-security-hardening|LICENSE
nemotron-speech|nvidia-speech|https://github.com/nvidia-riva/Nemotron-speech-skills.git|d41a75082063692b2fbd7f98202354b665f019f5|skills/nemotron-speech|LICENSE
frontend-design|anthropic-skills|https://github.com/anthropics/skills.git|2c5958ecac7cee2b583967a2c4981d0f4a75c579|skills/frontend-design|skills/frontend-design/LICENSE.txt
playwright-cli|playwright-cli|https://github.com/microsoft/playwright-cli.git|4bc00fb7fa5b4329f2fb53ac4743a066c2c664e8|skills/playwright-cli|LICENSE
test-driven-development|superpowers|https://github.com/obra/superpowers.git|22f0b463e7a5e579904dd6211a520321035526f7|skills/test-driven-development|LICENSE
systematic-debugging|superpowers|https://github.com/obra/superpowers.git|22f0b463e7a5e579904dd6211a520321035526f7|skills/systematic-debugging|LICENSE
verification-before-completion|superpowers|https://github.com/obra/superpowers.git|22f0b463e7a5e579904dd6211a520321035526f7|skills/verification-before-completion|LICENSE
property-based-testing|trailofbits|https://github.com/trailofbits/skills.git|e4dd0bb5b15535f7c5a0834187dffba6ac37e4ff|plugins/property-based-testing/skills/property-based-testing|LICENSE'

[[ $# -le 1 ]] || die "Pass one project directory, --dry-run, or --help."
case "${1:-.}" in
  --help|-h)
    printf '%s\n' \
      'Usage: bash firstring-codex-setup.sh [PROJECT_DIRECTORY | --dry-run]' \
      'Requires Bash, Git, tar, awk, diff, cmp, and access to public GitHub repos.' \
      'Keep firstring-codex-AGENTS.md beside this script.' \
      'Existing different skills cause a safe stop; identical skills are preserved.' \
      'Existing AGENTS.md is never overwritten. No global configuration is changed.'
    exit 0
    ;;
  --dry-run)
    printf 'Selected project-local skills (no files changed):\n'
    while IFS='|' read -r name cache repo pin source license; do
      printf '  %-32s %s @ %s\n' "$name" "$repo" "$pin"
    done <<< "$MANIFEST"
    printf '\nNo runtime packages, hooks, MCP servers, model calls, or phone calls.\n'
    exit 0
    ;;
  --*) die "Unknown option: $1" ;;
esac

for command in git tar awk diff cmp mktemp cp mv mkdir; do
  command -v "$command" >/dev/null 2>&1 || die "Missing required command: $command"
done

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
TEMPLATE="$SCRIPT_DIR/firstring-codex-AGENTS.md"
[[ -f "$TEMPLATE" ]] || die "Keep firstring-codex-AGENTS.md beside this script."
[[ -d "${1:-.}" ]] || die "Create your project directory first."
ROOT="$(cd -- "${1:-.}" && pwd -P)"
[[ "$ROOT" != / && "$ROOT" != "${HOME:-}" ]] || die "Choose a project folder, not / or your home directory."
[[ -w "$ROOT" ]] || die "Project directory is not writable."

for path in "$ROOT/.agents" "$ROOT/.agents/skills"; do
  [[ ! -L "$path" ]] || die "Refusing a symlinked installation directory: $path"
  [[ ! -e "$path" || -d "$path" ]] || die "Expected a directory: $path"
done

WORK="$(mktemp -d "${TMPDIR:-/tmp}/firstring-skills.XXXXXXXX")"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/repos" "$WORK/staged" "$WORK/export"
printf '%s\n' "$MANIFEST" > "$WORK/firstring-skills.lock"

LOCK="$ROOT/.agents/firstring-skills.lock"
[[ ! -L "$LOCK" ]] || die "Refusing a symlinked lockfile."
if [[ -e "$LOCK" ]] && ! cmp -s "$LOCK" "$WORK/firstring-skills.lock"; then
  die "An existing FirstRing lockfile differs. Review an update explicitly; nothing installed."
fi

# Stage every source before changing the destination.
while IFS='|' read -r name cache repo pin source license; do
  checkout="$WORK/repos/$cache"
  if [[ ! -d "$checkout" ]]; then
    printf 'Fetching pinned source: %s\n' "$repo"
    git -c init.templateDir= init --quiet "$checkout"
    GIT_TERMINAL_PROMPT=0 git -C "$checkout" -c core.hooksPath=/dev/null \
      fetch --quiet --no-tags --depth=1 "$repo" "$pin"
    actual="$(git -C "$checkout" rev-parse FETCH_HEAD)"
    [[ "$actual" == "$pin" ]] || die "Commit mismatch for $repo"
  fi

  git -C "$checkout" cat-file -e "$pin:$source/SKILL.md" \
    || die "Missing SKILL.md for $name"
  git -C "$checkout" cat-file -e "$pin:$license" \
    || die "Missing upstream license for $name"

  if git -C "$checkout" ls-tree -r "$pin" -- "$source" "$license" \
    | awk '$1 == "120000" || $1 == "160000" { bad=1 } END { exit (bad ? 0 : 1) }'; then
    die "Refusing symlinks or submodules in selected upstream content: $name"
  fi

  export_dir="$WORK/export/$name"
  mkdir -p "$export_dir"
  git -C "$checkout" archive "$pin" "$source" "$license" \
    | tar -xf - -C "$export_dir"
  cp -R "$export_dir/$source" "$WORK/staged/$name"
  cp "$export_dir/$license" "$WORK/staged/$name/UPSTREAM-LICENSE.txt"
  actual_name="$(awk '/^name:/ {
    sub(/^name:[[:space:]]*/, "");
    sub(/[[:space:]]*$/, "");
    sub(/^[\042\047]/, "");
    sub(/[\042\047]$/, "");
    print; exit
  }' "$WORK/staged/$name/SKILL.md")"
  [[ "$actual_name" == "$name" ]] || die "Unexpected skill name for $name: $actual_name"

  destination="$ROOT/.agents/skills/$name"
  [[ ! -L "$destination" ]] || die "Refusing existing symlink: $destination"
  if [[ -e "$destination" ]]; then
    [[ -d "$destination" ]] || die "Existing path is not a skill directory: $destination"
    diff -qr "$WORK/staged/$name" "$destination" >/dev/null \
      || die "Existing skill differs: $name. Nothing installed; review it manually."
  fi
done <<< "$MANIFEST"

mkdir -p "$ROOT/.agents/skills"
while IFS='|' read -r name cache repo pin source license; do
  destination="$ROOT/.agents/skills/$name"
  if [[ -d "$destination" ]]; then
    printf 'Preserved identical skill: %s\n' "$name"
  else
    mv "$WORK/staged/$name" "$destination"
    printf 'Installed: %s\n' "$name"
  fi
done <<< "$MANIFEST"
if [[ ! -e "$LOCK" ]]; then
  cp "$WORK/firstring-skills.lock" "$LOCK"
fi

if [[ -e "$ROOT/AGENTS.md" || -L "$ROOT/AGENTS.md" ]]; then
  printf 'Preserved existing AGENTS.md. Merge firstring-codex-AGENTS.md before development.\n'
else
  cp "$TEMPLATE" "$ROOT/AGENTS.md"
  printf 'Created project AGENTS.md.\n'
fi

BLUEPRINT="$SCRIPT_DIR/FirstRing_Master_Guide.md"
if [[ -f "$BLUEPRINT" && ! -L "$ROOT/docs" ]]; then
  if [[ ! -e "$ROOT/docs" || -d "$ROOT/docs" ]]; then
    mkdir -p "$ROOT/docs"
    if [[ ! -e "$ROOT/docs/FirstRing_Master_Guide.md" && ! -L "$ROOT/docs/FirstRing_Master_Guide.md" ]]; then
      cp "$BLUEPRINT" "$ROOT/docs/FirstRing_Master_Guide.md"
      printf 'Copied master guide to docs/FirstRing_Master_Guide.md.\n'
    fi
  fi
fi

printf '\nSkill files prepared in %s/.agents/skills\n' "$ROOT"
printf 'Confirm skill discovery in Codex; paste FirstRing_Codex_Start.md to audit and begin implementation.\n'
printf 'No provider accounts, runtimes, APIs, tunnels, phone calls, or global settings were changed.\n'
