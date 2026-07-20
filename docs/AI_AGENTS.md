# 🤖 AI Agents & GitHub Copilot — Enterprise Guide

**🇵🇱 Kompletny przewodnik po warstwie AI tego repozytorium: co jest skonfigurowane, jak i kiedy działa, jak tego używać w VS Code / na GitHub.com / w Copilot CLI, oraz jak to utrzymywać na poziomie enterprise.**

**🇬🇧 The complete guide to this repository's AI layer: what is configured, how and when it activates, how to use it in VS Code / on GitHub.com / in Copilot CLI, and how to maintain it at an enterprise level.**

> **🇵🇱** Repozytorium jest **AI-ready**: instrukcje repo, reguły per-ścieżka, prompty wielokrotnego użytku, standard cross-tool `AGENTS.md`, deterministyczny setup agenta chmurowego i konfiguracja VS Code — wszystko wersjonowane w Git i współdzielone z zespołem.
>
> **🇬🇧** This repository is **AI-ready**: repo instructions, path-scoped rules, reusable prompts, the cross-tool `AGENTS.md` standard, a deterministic cloud-agent setup and VS Code configuration — all version-controlled in Git and shared across the team.

---

## 📑 Spis treści / Table of contents

1. [Przegląd plików / File overview](#1--przegląd-plików--file-overview)
2. [Jak warstwy współgrają / How the layers combine](#2--jak-warstwy-współgrają--how-the-layers-combine)
3. [Szczegóły plików / File reference](#3--szczegóły-plików--file-reference)
4. [Scenariusze użycia / Usage by surface](#4--scenariusze-użycia--usage-by-surface)
5. [Ład korporacyjny i bezpieczeństwo / Enterprise governance & security](#5--ład-korporacyjny-i-bezpieczeństwo--enterprise-governance--security)
6. [Utrzymanie i dobre praktyki / Maintenance & best practices](#6--utrzymanie-i-dobre-praktyki--maintenance--best-practices)
7. [Rozwiązywanie problemów / Troubleshooting](#7--rozwiązywanie-problemów--troubleshooting)
8. [Źródła (dokumentacja oficjalna) / References (official docs)](#8--źródła-dokumentacja-oficjalna--references-official-docs)

---

## 1. 📁 Przegląd plików / File overview

**🇵🇱** Poniżej wszystkie pliki warstwy AI w tym repo, ich rola i narzędzie, które je czyta. **🇬🇧** All AI-layer files in this repo, their role, and the tool that consumes them.

| Plik / File | Rola / Role | Kto czyta / Consumed by |
|---|---|---|
| [`.github/copilot-instructions.md`](../.github/copilot-instructions.md) | Instrukcje repo (stack, komendy, bramki, konwencje) / Repo-wide instructions | GitHub Copilot (wszędzie / everywhere) |
| [`.github/instructions/rust.instructions.md`](../.github/instructions/rust.instructions.md) | Reguły `applyTo: src-tauri/**/*.rs` / Path-scoped Rust rules | Copilot (path-scoped) |
| [`.github/instructions/frontend.instructions.md`](../.github/instructions/frontend.instructions.md) | Reguły `applyTo: src/**` / Path-scoped frontend rules | Copilot (path-scoped) |
| [`AGENTS.md`](../AGENTS.md) | Standard cross-tool dla agentów / Cross-tool agent standard | Copilot coding agent, CLI, inne agenty / other agents |
| [`.github/prompts/*.prompt.md`](../.github/prompts) | Prompty wielokrotnego użytku (slash-commands) / Reusable prompts | Copilot Chat / CLI |
| [`.github/copilot-setup-steps.yml`](../.github/workflows/copilot-setup-steps.yml) | Środowisko agenta chmurowego / Cloud-agent environment | Copilot coding agent |
| [`.github/pull_request_template.md`](../.github/pull_request_template.md) | Szablon PR z checklistą bramek / PR template with gates | Ludzie + agent / Humans + agent |
| [`.vscode/settings.json`](../.vscode/settings.json) | Włącza pliki instrukcji/promptów / Enables instruction & prompt files | VS Code Copilot |
| [`.vscode/extensions.json`](../.vscode/extensions.json) | Rekomendowane rozszerzenia / Recommended extensions | VS Code |

---

## 2. 🧩 Jak warstwy współgrają / How the layers combine

**🇵🇱** GitHub Copilot obsługuje **trzy** typy instrukcji repo. Nie wykluczają się — są **łączone**:
- **repo-wide** (`.github/copilot-instructions.md`) — do każdego żądania w kontekście repo,
- **path-specific** (`.github/instructions/*.instructions.md`) — gdy edytowany plik pasuje do `applyTo`; wtedy reguły repo-wide **i** path-specific są używane **razem**,
- **agent** (`AGENTS.md`) — dla agentów; przy wielu plikach `AGENTS.md` w drzewie **najbliższy** ma pierwszeństwo.

**🇬🇧** GitHub Copilot supports **three** kinds of repository instructions. They don't compete — they are **combined**:
- **repo-wide** (`.github/copilot-instructions.md`) — for every request in the repo context,
- **path-specific** (`.github/instructions/*.instructions.md`) — when the file in context matches `applyTo`; repo-wide **and** path-specific rules are then used **together**,
- **agent** (`AGENTS.md`) — for agents; with multiple `AGENTS.md` files in the tree, the **nearest** one takes precedence.

```text
                 ┌─────────────────────────────────────────────┐
 Request (Rust)  │  copilot-instructions.md   (repo-wide)       │
        │        │            +                                 │
        ▼        │  instructions/rust.instructions.md (applyTo) │  ← combined context
 src-tauri/*.rs  │            +                                 │
                 │  AGENTS.md (nearest)  +  your personal prefs │
                 └─────────────────────────────────────────────┘
```

> **🇵🇱** Zasada: instrukcje krótkie, konkretne, niesprzeczne. Konflikty i „lanie wody" pogarszają wyniki modelu.
> **🇬🇧** Rule of thumb: keep instructions short, concrete and non-contradictory. Conflicts and filler degrade model output.

---

## 3. 📘 Szczegóły plików / File reference

### 3.1 `.github/copilot-instructions.md` — instrukcje repo / repo-wide instructions

**🇵🇱**
- **Co:** jeden plik z opisem projektu, stacku, układu katalogów, komend i **bramek jakości** (build/test/lint) oraz konwencji (bezpieczny scan, model gałęzi, etykiety w `metadata`).
- **Kiedy działa:** automatycznie, przy każdym żądaniu Copilota w kontekście repo (VS Code, GitHub.com, CLI, code review).
- **Jak używać:** nic nie robisz — Copilot wczytuje go sam. Edytuj, gdy zmieniają się komendy/konwencje.

**🇬🇧**
- **What:** a single file describing the project, stack, directory layout, commands and **quality gates**, plus conventions (no-panic scan, rung model, labels in `metadata`).
- **When:** automatically, on every Copilot request in the repo context (VS Code, GitHub.com, CLI, code review).
- **How to use:** nothing to do — Copilot loads it automatically. Edit it when commands/conventions change.

📖 [Adding repository custom instructions (GitHub Docs)](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions) · [Custom instructions (VS Code)](https://code.visualstudio.com/docs/agent-customization/custom-instructions)

### 3.2 `.github/instructions/*.instructions.md` — reguły per-ścieżka / path-scoped rules

**🇵🇱**
- **Co:** pliki z nagłówkiem YAML `applyTo: "<glob>"`, które stosują się **tylko** do pasujących plików. Mamy `rust.instructions.md` (`src-tauri/**/*.rs`) i `frontend.instructions.md` (`src/**`).
- **Kiedy działa:** gdy Copilot pracuje nad plikiem pasującym do `applyTo` — dokładane do instrukcji repo-wide.
- **Jak używać:** dodawaj kolejne pliki dla nowych obszarów (np. testy, workflow). Trzymaj `applyTo` wąsko.

**🇬🇧**
- **What:** files with a YAML header `applyTo: "<glob>"` that apply **only** to matching files. We ship `rust.instructions.md` (`src-tauri/**/*.rs`) and `frontend.instructions.md` (`src/**`).
- **When:** when Copilot works on a file matching `applyTo` — added on top of the repo-wide instructions.
- **How to use:** add more files for new areas (e.g. tests, workflows). Keep `applyTo` narrow.

```markdown
---
applyTo: "src-tauri/**/*.rs"
---
# Rules that only apply to Rust files…
```

📖 [Path-specific custom instructions (GitHub Docs)](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions) · [Instructions files (VS Code)](https://code.visualstudio.com/docs/agent-customization/custom-instructions)

### 3.3 `AGENTS.md` — standard cross-tool / cross-tool agent standard

**🇵🇱**
- **Co:** otwarty standard (`agents.md`) czytany przez wielu agentów AI: GitHub Copilot coding agent, Copilot CLI, oraz narzędzia jak Cursor/Codex. U nas w katalogu głównym.
- **Kiedy działa:** gdy agent pracuje w repo; przy wielu `AGENTS.md` **najbliższy w drzewie** wygrywa. Alternatywy: `CLAUDE.md`, `GEMINI.md` w roocie.
- **Jak używać:** utrzymuj spójnie z `copilot-instructions.md` (u nas `AGENTS.md` linkuje do niego, by uniknąć rozjazdu).

**🇬🇧**
- **What:** an open standard (`agents.md`) read by many AI agents: GitHub Copilot coding agent, Copilot CLI, and tools like Cursor/Codex. Located at the repo root here.
- **When:** when an agent works in the repo; with multiple `AGENTS.md`, the **nearest in the tree** wins. Alternatives: `CLAUDE.md`, `GEMINI.md` at the root.
- **How to use:** keep it consistent with `copilot-instructions.md` (ours links to it to avoid drift).

📖 [agents.md](https://agents.md/) · [agentsmd/agents.md repo](https://github.com/agentsmd/agents.md)

### 3.4 `.github/prompts/*.prompt.md` — prompty wielokrotnego użytku / reusable prompts

**🇵🇱**
- **Co:** zapisane prompty z nagłówkiem `mode`/`description`, wywoływane jako **slash-command**. Mamy: `new-ladder-instruction`, `verify-changes`, `review-changes`.
- **Kiedy działa:** **na żądanie** — nie automatycznie. Wpisujesz `/nazwa` w Copilot Chat / CLI.
- **Jak używać (VS Code):** w Copilot Chat wpisz np. `/verify-changes`. Można przekazać zmienne `${input:...}`.

**🇬🇧**
- **What:** saved prompts with a `mode`/`description` header, invoked as a **slash command**. We ship `new-ladder-instruction`, `verify-changes`, `review-changes`.
- **When:** **on demand** — not automatic. Type `/name` in Copilot Chat / CLI.
- **How to use (VS Code):** in Copilot Chat type e.g. `/verify-changes`. You can pass `${input:...}` variables.

```text
/new-ladder-instruction   → scaffold a new IEC instruction package
/verify-changes           → run every quality gate and report pass/fail
/review-changes           → review the current diff against project conventions
```

📖 [Prompt files (VS Code)](https://code.visualstudio.com/docs/agent-customization/prompt-files)

### 3.5 `.github/copilot-setup-steps.yml` — środowisko agenta chmurowego / cloud-agent environment

**🇵🇱**
- **Co:** workflow GitHub Actions z **jednym** jobem o wymaganej nazwie `copilot-setup-steps`. Preinstaluje Node, Rust, zależności systemowe Tauri i rozgrzewa cache `cargo`.
- **Kiedy działa:** uruchamiany **zanim** agent chmurowy (Copilot coding agent) zacznie pracę. **Musi być na gałęzi domyślnej**, aby zadziałał. Można go też odpalić ręcznie w zakładce **Actions**.
- **Jak używać:** aktualizuj kroki, gdy zmieniają się zależności. Ustawiaj minimalne `permissions`.

**🇬🇧**
- **What:** a GitHub Actions workflow with a **single** job that must be named `copilot-setup-steps`. It preinstalls Node, Rust, Tauri system deps and warms the `cargo` cache.
- **When:** runs **before** the cloud agent (Copilot coding agent) starts. **Must be on the default branch** to take effect. Can also be run manually from the **Actions** tab.
- **How to use:** update the steps when dependencies change. Keep `permissions` minimal.

📖 [Customize the agent environment (GitHub Docs)](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/customize-the-agent-environment)

### 3.6 `.github/pull_request_template.md` — szablon PR / PR template

**🇵🇱** **Co/Kiedy:** automatycznie wypełnia opis PR na GitHub.com (dla ludzi **i** dla PR-ów agenta chmurowego). Zawiera checklistę bramek jakości. **Jak:** uzupełnij podsumowanie i odhacz bramki przed merge.

**🇬🇧** **What/When:** pre-fills the PR description on GitHub.com (for humans **and** cloud-agent PRs). Contains the quality-gate checklist. **How:** fill the summary and tick the gates before merge.

### 3.7 `.vscode/settings.json` + `.vscode/extensions.json` — konfiguracja VS Code / VS Code config

**🇵🇱**
- **settings.json:** włącza pliki instrukcji (`github.copilot.chat.codeGeneration.useInstructionFiles`) i promptów (`chat.promptFiles`), wskazuje `rust-analyzer.linkedProjects` na `src-tauri/Cargo.toml` (crate nie jest w roocie).
- **extensions.json:** rekomenduje rozszerzenia (Copilot, rust-analyzer, Svelte, Tauri) — VS Code zaproponuje ich instalację.
- **Uwaga:** te dwa pliki są **odsłonięte** w `.gitignore` (`!.vscode/settings.json`), reszta `.vscode/` pozostaje ignorowana.

**🇬🇧**
- **settings.json:** enables instruction files (`github.copilot.chat.codeGeneration.useInstructionFiles`) and prompt files (`chat.promptFiles`), and points `rust-analyzer.linkedProjects` at `src-tauri/Cargo.toml` (the crate is not at the root).
- **extensions.json:** recommends extensions (Copilot, rust-analyzer, Svelte, Tauri) — VS Code offers to install them.
- **Note:** these two files are **un-ignored** in `.gitignore` (`!.vscode/settings.json`); the rest of `.vscode/` stays ignored.

📖 [Agent customization overview (VS Code)](https://code.visualstudio.com/docs/agent-customization/overview)

---

## 4. 🛠️ Scenariusze użycia / Usage by surface

### 4.1 VS Code (Copilot Chat / Agent)

**🇵🇱**
1. Zainstaluj rekomendowane rozszerzenia (VS Code zaproponuje z `extensions.json`).
2. Instrukcje repo + path-scoped ładują się **automatycznie**.
3. Uruchom prompty przez `/verify-changes`, `/review-changes`, `/new-ladder-instruction` w oknie Chat.
4. Chcesz wygenerować nowy plik customizacji? Wpisz `/create-instruction` lub `/create-prompt`.

**🇬🇧**
1. Install the recommended extensions (VS Code prompts you from `extensions.json`).
2. Repo + path-scoped instructions load **automatically**.
3. Run prompts via `/verify-changes`, `/review-changes`, `/new-ladder-instruction` in the Chat view.
4. Want to generate a new customization file? Type `/create-instruction` or `/create-prompt`.

### 4.2 GitHub.com — Copilot coding agent (agent chmurowy)

**🇵🇱** Przypisz Copilotowi issue lub poproś o zmianę na GitHub.com. Agent uruchomi `copilot-setup-steps.yml`, wczyta `copilot-instructions.md` + `instructions/**` + `AGENTS.md`, wykona pracę i otworzy PR (z naszym szablonem). Sprawdź logi sesji w zakładce Actions/PR.

**🇬🇧** Assign an issue to Copilot or request a change on GitHub.com. The agent runs `copilot-setup-steps.yml`, loads `copilot-instructions.md` + `instructions/**` + `AGENTS.md`, does the work and opens a PR (using our template). Review the session logs in the Actions/PR view.

### 4.3 Copilot CLI

**🇵🇱** W tym terminalu (Copilot CLI) agent czyta `AGENTS.md` i instrukcje repo, a prompty z `.github/prompts` są dostępne jako komendy. Idealne do bramek jakości i scaffoldingu.

**🇬🇧** In this terminal (Copilot CLI) the agent reads `AGENTS.md` and the repo instructions, and the prompts in `.github/prompts` are available as commands. Great for quality gates and scaffolding.

### 4.4 Inne modele/agenty AI / Other AI models & agents

**🇵🇱** Narzędzia zgodne ze standardem `AGENTS.md` (np. Cursor, Codex i inne) automatycznie skorzystają z `AGENTS.md`. Dla Claude/Gemini można dodać `CLAUDE.md`/`GEMINI.md` w roocie (opcjonalnie linkujące do `AGENTS.md`).

**🇬🇧** Tools compatible with the `AGENTS.md` standard (e.g. Cursor, Codex and others) will pick up `AGENTS.md` automatically. For Claude/Gemini you can add a root `CLAUDE.md`/`GEMINI.md` (optionally pointing to `AGENTS.md`).

---

## 5. 🔐 Ład korporacyjny i bezpieczeństwo / Enterprise governance & security

**🇵🇱**
- **Sekrety/zmienne agenta chmurowego:** ustawiaj w środowisku `copilot` repo (Settings → Environments → `copilot`), nie w plikach. Nigdy nie commituj kluczy.
- **Najmniejsze uprawnienia:** `copilot-setup-steps.yml` ma `permissions: contents: read`. Nie rozszerzaj bez potrzeby.
- **Firewall agenta:** domyślny firewall Copilota ogranicza ruch sieciowy; przy self-hosted/Windows wymagana osobna konfiguracja sieci.
- **Przegląd:** PR agenta przechodzi normalny code review + CI (fmt, clippy `-D warnings`, testy, `cargo audit`). Zasada „człowiek zatwierdza merge".
- **Bezpieczeństwo produktu:** nie osłabiaj domyślnych ustawień (Modbus localhost + bramka zapisu, minimalne capabilities Tauri, łańcuch audytu). Reguły są w `instructions/rust.instructions.md`.

**🇬🇧**
- **Cloud-agent secrets/variables:** set them in the repo's `copilot` environment (Settings → Environments → `copilot`), not in files. Never commit keys.
- **Least privilege:** `copilot-setup-steps.yml` uses `permissions: contents: read`. Don't widen it without a reason.
- **Agent firewall:** Copilot's default firewall limits network egress; self-hosted/Windows runners need their own network controls.
- **Review:** agent PRs go through normal code review + CI (fmt, clippy `-D warnings`, tests, `cargo audit`). A human approves the merge.
- **Product security:** don't weaken the defaults (Modbus localhost + write gate, minimal Tauri capabilities, audit chain). The rules live in `instructions/rust.instructions.md`.

📖 [Customize/disable the firewall (GitHub Docs)](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/customize-the-agent-firewall) · [Response customization (concepts)](https://docs.github.com/en/copilot/concepts/prompting/response-customization)

---

## 6. ♻️ Utrzymanie i dobre praktyki / Maintenance & best practices

**🇵🇱**
- **Jedno źródło prawdy:** komendy i bramki utrzymuj w `copilot-instructions.md`; `AGENTS.md` i reguły path-scoped niech się do niego odwołują, bez duplikacji.
- **Krótko i konkretnie:** unikaj sprzeczności i długich akapitów — psują wyniki modelu.
- **Waliduj:** frontmatter (`applyTo`, `mode`) musi być poprawnym YAML; JSON w `.vscode` — poprawnym JSON.
- **Testuj po zmianie:** po edycji `copilot-setup-steps.yml` odpal workflow ręcznie (Actions) i sprawdź, czy przechodzi.
- **Ewolucja:** dodawaj nowe reguły dopiero, gdy pojawia się powtarzalna potrzeba.

**🇬🇧**
- **Single source of truth:** keep commands and gates in `copilot-instructions.md`; have `AGENTS.md` and path-scoped rules reference it instead of duplicating.
- **Short and concrete:** avoid contradictions and long paragraphs — they hurt model output.
- **Validate:** frontmatter (`applyTo`, `mode`) must be valid YAML; JSON in `.vscode` must be valid JSON.
- **Test after changes:** after editing `copilot-setup-steps.yml`, run the workflow manually (Actions) and confirm it passes.
- **Evolve:** add new rules only when a recurring need appears.

---

## 7. 🩺 Rozwiązywanie problemów / Troubleshooting

| Objaw / Symptom | Przyczyna i rozwiązanie / Cause & fix |
|---|---|
| Copilot ignoruje instrukcje / ignores instructions | **🇵🇱** W VS Code włącz `github.copilot.chat.codeGeneration.useInstructionFiles` (jest w `settings.json`). **🇬🇧** In VS Code enable `github.copilot.chat.codeGeneration.useInstructionFiles` (set in `settings.json`). |
| Prompty `/...` niewidoczne / prompts not showing | **🇵🇱** Włącz `chat.promptFiles: true`; pliki muszą być w `.github/prompts` z rozszerzeniem `.prompt.md`. **🇬🇧** Enable `chat.promptFiles: true`; files must be in `.github/prompts` with a `.prompt.md` extension. |
| `copilot-setup-steps` się nie uruchamia / doesn't run | **🇵🇱** Plik musi być na **gałęzi domyślnej**, a job nazwany **dokładnie** `copilot-setup-steps`. **🇬🇧** The file must be on the **default branch** and the job named **exactly** `copilot-setup-steps`. |
| Reguły path-scoped nie działają / path rules ignored | **🇵🇱** Sprawdź `applyTo` (glob) i czy edytowany plik pasuje. **🇬🇧** Check `applyTo` (glob) and that the edited file matches. |
| `.vscode/*` nie commituje się / won't commit | **🇵🇱** To zamierzone — odsłonięte są tylko `settings.json`/`extensions.json` przez `!` w `.gitignore`. **🇬🇧** Intentional — only `settings.json`/`extensions.json` are un-ignored via `!` in `.gitignore`. |

---

## 8. 🔗 Źródła (dokumentacja oficjalna) / References (official docs)

**GitHub Copilot**
- [Adding repository custom instructions](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions)
- [About customizing Copilot responses (concepts)](https://docs.github.com/en/copilot/concepts/prompting/response-customization)
- [Customize the cloud-agent environment (`copilot-setup-steps`)](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/customize-the-agent-environment)
- [Customize / disable the agent firewall](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/customize-the-agent-firewall)

**VS Code**
- [Agent customization — overview](https://code.visualstudio.com/docs/agent-customization/overview)
- [Custom instructions](https://code.visualstudio.com/docs/agent-customization/custom-instructions)
- [Prompt files](https://code.visualstudio.com/docs/agent-customization/prompt-files)
- [Custom agents](https://code.visualstudio.com/docs/agent-customization/custom-agents)
- [MCP servers](https://code.visualstudio.com/docs/agent-customization/mcp-servers)

**Standard cross-tool / GitHub Actions**
- [AGENTS.md standard](https://agents.md/) · [agentsmd/agents.md](https://github.com/agentsmd/agents.md)
- [GitHub Actions — workflow syntax](https://docs.github.com/en/actions/reference/workflow-syntax-for-github-actions)

---

<div align="center">

**🇵🇱 Utrzymuj ten dokument aktualnym wraz z warstwą AI repo. 🇬🇧 Keep this document in sync with the repo's AI layer.**

</div>
