# FirstRing: ready-to-start Codex package

This combines the master project guide, the eleven-skill starter kit and one starting prompt. It prepares the coding handoff; no FirstRing application or provider integration has been implemented by this package.

## Start

1. Put this folder's five files in your FirstRing repository root. Preserve any existing files with the same names and compare before replacing them. The files are already named correctly for the installer.
2. Review `firstring-codex-setup.sh`, then run the commands below from the repository root as separate shell commands.
3. Open `FirstRing_Codex_Start.md` and paste the prompt below its introductory separator into Codex. The master guide must remain available to it. Do not also paste the old initialization-only or first-development prompts.

```bash
bash ./firstring-codex-setup.sh --dry-run
bash ./firstring-codex-setup.sh .
codex
```

Use your available model/reasoning selection. If Astra/Ultra is offered by your client, you can select it there; this package does not alter global model configuration. Confirm skills in the client; restart/reload it if repository skills are not visible. Skill loading conventions are documented in [Codex skills](https://developers.openai.com/codex/skills).

If you prefer to let Codex handle setup, put the files in the repository and paste the unified prompt directly. It explicitly authorizes reviewing and running this project-local installer, then continuing into code. Do not run installation twice unnecessarily.

## Included files

| File | Purpose |
|---|---|
| `FirstRing_Codex_Start.md` | One prompt: audit, implement, verify, continue through milestones. |
| `FirstRing_Master_Guide.md` | Full product and architecture specification, demos, API references and business preparation. |
| `firstring-codex-AGENTS.md` | Updated persistent project contract; merge if an `AGENTS.md` already exists. |
| `firstring-codex-setup.sh` | Original eleven pinned skill selections, corrected master-guide filename and completion instructions. |
| `README.md` | These instructions. |

The installer downloads skill folders and licenses to `.agents/skills`, records pins, preserves existing different files, and copies the guide to `docs/FirstRing_Master_Guide.md` only if that destination is absent. The prompt reads the docs copy first; reconcile an older docs copy if you already have one. The script does not install app dependencies, browser runtimes or global tools, and does not call providers.

## What changed in the merge

- Initialization now leads directly into implementation. The first checkpoint stays deliberately small: two real phone legs and DTMF, without a model or conference.
- The next conversational milestone uses ConversationRelay and Nemotron; Gather remains a setup-blocked fallback.
- The full build retains the revocable private handoff, independent trusted contact, real UI events and second-household tactic update.
- Skills are loaded only for relevant work. Test-first guidance cannot delete existing work or force tests for trivial documentation/configuration changes.
- Existing authorization persists. Setup blockers are named explicitly while independent development continues.
- Files are correctly named, shell commands are separated, and the older blueprint no longer silently replaces the master guide.

## Source review and validation boundary

The eleven manifest entries retain the exact commit pins from the supplied installer. GitHub reads confirmed the corresponding `SKILL.md` files and declared names at those revisions. Targeted full-content review covered the Twilio voice, NVIDIA speech and Superpowers TDD skills. Bash syntax and no-write dry-run checks were performed for this merged package; see the delivered summary for their outcomes.

The supplied report's full clean-install/repeat-run/Codex-discovery tests were not repeated here. Actual installation, supporting-file checks, client discovery, browser runtime and provider access still need verification on your machine. A pin makes files reproducible, not automatically correct.

The separate Twilio ConversationRelay skill was reviewed but not added: its pinned message table differs from the current [Relay protocol](https://www.twilio.com/docs/voice/conversationrelay/websocket-messages). The unified prompt therefore uses primary API documentation for Relay rather than expanding the skill bundle. It also keeps [NVIDIA speech guidance](https://github.com/nvidia-riva/Nemotron-speech-skills/blob/d41a75082063692b2fbd7f98202354b665f019f5/skills/nemotron-speech/SKILL.md) separate from LLM tool calling.

Keep the vendored skills, license files and pin manifest under the repository's normal version control. Do not commit credentials, provider logs or private audio.
