## JDK-Pulse – Implementation Plan

This plan is organized into **phases** that roughly match the milestones in `SPECIFICATION.md`, with concrete, buildable steps.

---

### Phase 1 – Core Backend & macOS JDK Discovery

**Goal**: Have a working Rust backend that can discover installed JDKs on macOS and expose them via a simple API (CLI and later Tauri).

- **1.1 Project layout**
  - Create `src-tauri/` as the Rust backend crate (Tauri-compatible structure).
  - Add `src-tauri/Cargo.toml` and `src-tauri/src/main.rs`.
  - Define shared data structures:
    - `JdkInfo`
    - `DoctorReport` (stub for now)

- **1.2 macOS JDK detection (CLI first)**
  - Implement `list_jdks()` in Rust (macOS only for now):
    - Call `/usr/libexec/java_home -V`.
    - Parse output into a `Vec<JdkInfo>`.
    - Print JSON to stdout (temporary CLI interface).
  - On non-macOS platforms, return an empty list or a clear “not implemented” message.

- **1.3 Canonical state write**
  - Add a function `set_active_jdk(home: &str)` that:
    - Validates the path.
    - Writes it into `~/.jdk_current` (macOS/Linux path; no Windows yet).
  - Provide a CLI subcommand or flag to:
    - `--list` (print all JDKs as JSON).
    - `--set <home>` (set `~/.jdk_current`).

> Output of Phase 1: a small Rust binary you can run locally that lists JDKs and sets the canonical JDK state file on macOS.

---

### Phase 2 – Tauri Shell & Tray Integration (macOS)

**Goal**: Wrap the backend in a minimal Tauri app with a tray icon and JDK selection menu on macOS.

- **2.1 Tauri bootstrapping**
  - Initialize a Tauri v2 app in this repo, wiring it to use `src-tauri` as the backend crate.
  - Configure system tray and disable unnecessary window chrome (tray-only app).

- **2.2 Expose Rust commands to Tauri**
  - Turn `list_jdks()` into a `#[tauri::command]`.
  - Add `set_active_jdk(id: String)` as a command (mapping `id` → `home`).

- **2.3 Tray UI**
  - Build a tray menu that:
    - Shows current JDK major version in the icon tooltip.
    - Lists available JDKs as radio items.
    - Calls `set_active_jdk` on selection.

> Output of Phase 2: a macOS tray app where selecting an entry updates `~/.jdk_current`.

---

### Phase 3 – Shell Injection (macOS zsh/bash)

**Goal**: Make existing and new terminals follow the selected JDK automatically.

- **3.1 Shell hook installer**
  - Implement `install_shell_integration()` in Rust:
    - Append a tagged `precmd`/`PROMPT_COMMAND` hook block into `~/.zshrc` (and optionally `~/.bashrc` / `~/.bash_profile`).
  - Implement `remove_shell_integration()` that removes those tagged blocks.

- **3.2 Integration with tray app**
  - Add settings UI in Tauri:
    - Button: “Install shell integration”.
    - Button: “Remove shell integration”.

> Output of Phase 3: change JDK in tray → press Enter in any terminal → `java -version` matches the selection.

---

### Phase 4 – Doctor Sync (macOS)

**Goal**: Verify and visualize environment alignment.

- **4.1 Doctor backend**
  - Implement `run_doctor_sync()` in Rust:
    - Spawn a login shell using user’s default shell.
    - Capture `$JAVA_HOME` and `java -version`.
    - Compare with `~/.jdk_current`.

- **4.2 Doctor UI**
  - Add “Doctor Sync” menu item or small window:
    - Show ✅/⚠️/❌ for:
      - State file vs shell environment.
      - State file vs `java -version`.

> Output of Phase 4: one-click diagnostics that tell you if the environment is consistent.

---

### Phase 5 – Windows Support

**Goal**: Provide equivalent JDK detection and switching on Windows.

- **5.1 JDK detection**
  - Implement Windows-specific detection:
    - Scan registry and common directories.

- **5.2 Environment update**
  - Update `JAVA_HOME` (User/System) via registry.
  - Broadcast `WM_SETTINGCHANGE`.

- **5.3 Shell integration**
  - PowerShell profile hook using `.jdk_current`.
  - Optional `cmd` helper script.

> Output of Phase 5: tray-based JDK selection working on Windows with environment updates.

---

### Phase 6 – Docker/Testcontainers & AI/IDE Integration

**Goal**: Close the loop with containers, build tools, and AI agents.

- **6.1 Docker/Testcontainers checks**
  - Extend Doctor Sync to:
    - Run `docker info`.
    - Optionally run a tiny Testcontainers probe (documented best-effort).

- **6.2 AI agent hints**
  - Generate `.cursor/rules/JAVA_HOME.md` and/or `~/.jdk-pulse/context.json`.
  - Add “Generate AI/Editor hints” to settings.

- **6.3 IDE snippets**
  - Provide snippets for:
    - VS Code `settings.json`.
    - IntelliJ project JDK (document-only or minimal automation).

> Output of Phase 6: richer diagnostics and smoother interoperability with AI tools and IDEs.

