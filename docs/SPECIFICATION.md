## JDK-Pulse – Technical Specification

### 1. Project Overview

- **Name**: JDK‑Pulse
- **Goal**: A professional‑grade, cross‑platform JDK switcher that keeps:
  - System environment
  - Terminals (including IDE‑embedded)
  - AI agents and build tools
  all aligned on the same Java version with one click.

- **Primary Platforms (initial focus)**:
  - macOS (Apple Silicon + Intel)
  - Windows 10/11
  - Linux (later milestone)

- **Tech Stack**:
  - **Shell App**: Tauri v2
  - **Backend**: Rust
  - **Frontend**: React + Tailwind + Shadcn UI (can be swapped if you prefer another UI lib)

---

### 2. Functional Requirements

#### 2.1 JDK Detection

- **FR-1**: Detect installed JDKs on the host system.
  - macOS:
    - Use `/usr/libexec/java_home -V` to enumerate available JDKs.
    - Optionally scan `/Library/Java/JavaVirtualMachines`.
  - Windows:
    - Scan common directories (`C:\Program Files\Java`, `C:\Program Files\Eclipse Adoptium`, etc.).
    - Read registry keys for JDK installations.
  - Linux:
    - Scan `/usr/lib/jvm`.
    - Optionally integrate with `update-alternatives`.

- **FR-2**: Normalize JDK metadata:
  - `id` (string, internal)
  - `version_major` (int: 8, 11, 17, 21, 25…)
  - `version_full` (string)
  - `home` (absolute path for `JAVA_HOME`)
  - `vendor` (optional)

#### 2.2 JDK Selection & Persistence

- **FR-3**: Allow the user to choose an active JDK from the tray menu.
- **FR-4**: Persist the selection:
  - Write canonical JDK home to:
    - `~/.jdk_current` (macOS/Linux)
    - `%USERPROFILE%\.jdk_current` (Windows)
  - Persist last selected JDK in app config (e.g. using Tauri’s config or a small JSON file).

#### 2.3 Environment Injection

- **FR-5**: Install optional shell hooks for:
  - `zsh` (primary on macOS)
  - `bash` (where applicable)
  - PowerShell (Windows)
  - Optional `cmd` helper

- **FR-6**: Ensure active shells sync from the canonical state file on each prompt/idle.

- **FR-7**: On Windows, update `JAVA_HOME` (User/System) and broadcast `WM_SETTINGCHANGE`.

#### 2.4 Visual Status & UX

- **FR-8**: Show the active JDK major version in the system tray/menu bar as a badge (e.g. `21`).
- **FR-9**: Provide a dropdown menu:
  - List of available JDKs with:
    - Version label (e.g. `Java 21 (Temurin)`)
    - Path preview on hover or in sub-menu
  - “Doctor Sync”
  - Settings / Preferences
  - Quit

#### 2.5 Doctor Sync

- **FR-10**: Run a diagnostic check that:
  - Compares:
    - `~/.jdk_current`
    - `JAVA_HOME`
    - `java -version` from a spawned shell using the user’s configured shell + profile.
  - Optionally checks:
    - Docker availability (`docker info`)
    - That JDK‑Pulse’s hooks are installed and active.

- **FR-11**: Return a structured report:

```json
{
  "state_file": { "path": "...", "java_home": "...", "ok": true },
  "shell_env": { "java_home": "...", "java_version": "...", "ok": true },
  "docker": { "available": true, "ok": true },
  "notes": ["..."]
}
```

The UI will render this as checkmarks / warnings.

#### 2.6 AI Agent & IDE Context Sync

- **FR-12**: Optionally generate/update a small configuration file that AI agents can read, e.g.:
  - User-level: `~/.jdk-pulse/context.json`
  - Project-level: `.jdkpulse.json` or `.cursor/rules/JAVA_HOME.md`.

- **FR-13**: Provide an option in the UI:
  - “Generate AI/Editor hints for this project…”
  - Writes context files into the selected project directory.

---

### 3. Non-Functional Requirements

- **NFR-1**: Low resource usage:
  - Idle CPU near zero; memory footprint minimal (Tauri/Rust help here).
- **NFR-2**: Safe configuration changes:
  - All modifications to shell configs and OS settings must be reversible.
- **NFR-3**: Cross-platform consistency:
  - Core behavior (select JDK, sync shells, show status) should be conceptually identical across OSes.
- **NFR-4**: Clear observability:
  - Log important events (detection, switching, injection errors) to a local log file.

---

### 4. Data Structures (Rust-side)

```rust
pub struct JdkInfo {
    pub id: String,
    pub version_major: u32,
    pub version_full: String,
    pub home: String,
    pub vendor: Option<String>,
}

pub struct DoctorReport {
    pub state_file_ok: bool,
    pub state_file_java_home: Option<String>,
    pub shell_java_home: Option<String>,
    pub shell_java_version: Option<String>,
    pub docker_available: Option<bool>,
    pub notes: Vec<String>,
}
```

These will be serialized to JSON and returned to the frontend via Tauri commands.

---

### 5. Tauri Commands (API Surface)

Initial set of commands:

```rust
#[tauri::command]
async fn list_jdks() -> Result<Vec<JdkInfo>, String>;

#[tauri::command]
async fn get_active_jdk() -> Result<Option<JdkInfo>, String>;

#[tauri::command]
async fn set_active_jdk(id: String) -> Result<(), String>;

#[tauri::command]
async fn run_doctor_sync() -> Result<DoctorReport, String>;

#[tauri::command]
async fn install_shell_integration() -> Result<(), String>;

#[tauri::command]
async fn remove_shell_integration() -> Result<(), String>;
```

Frontend calls these via Tauri’s JavaScript bindings.

---

### 6. Frontend (React) Sketch

- **SystemTray Setup**
  - Use Tauri’s system tray API to:
    - Create a tray icon with dynamic tooltip (e.g. `JDK-Pulse – Java 21`).
    - Populate menu items:
      - One per JDK (`Radio` style)
      - Doctor Sync
      - Settings
      - Quit

- **Settings Window**
  - Simple React window (Tailwind + Shadcn):
    - Tabs:
      - General
      - Shell Integration
      - AI/IDE Integration
      - About

---

### 7. Implementation Roadmap

#### Milestone 1 – macOS Minimal Viable App

- Tauri app with:
  - System tray icon.
  - `list_jdks` implementation via `java_home -V`.
  - Ability to select a JDK and write `~/.jdk_current`.
- Shell integration for `zsh`:
  - Install / remove a `precmd`-based hook.
- Simple Doctor Sync:
  - Spawn a shell to query `java -version` and `$JAVA_HOME`.

#### Milestone 2 – Windows Support

- JDK detection via registry and common paths.
- `JAVA_HOME` registry updates + `WM_SETTINGCHANGE`.
- PowerShell profile hook and optional CMD helper.

#### Milestone 3 – Docker / Testcontainers Awareness

- Doctor Sync checks:
  - `docker info`
  - Basic Testcontainers diagnostics (optional, may be just documented rather than automatic).

#### Milestone 4 – AI & IDE Integration

- Generate AI hint files for:
  - Cursor / `.cursor`-based rules
  - VS Code workspace settings override snippet
  - Simple docs for IntelliJ.

---

### 8. Next Steps (Practical)

For bootstrapping the project:

1. **Initialize Tauri app skeleton** in this repo:
   - Use `npm create tauri-app` or the Rust‑only flow, depending on your preference.
2. **Implement macOS JDK detection** in Rust with a small test harness.
3. **Wire the tray menu** to `list_jdks` / `set_active_jdk`.
4. **Add the shell hook installer** for `zsh` and validate:
   - Select JDK in tray → open new terminal → `java -version` matches.

Once this is working, we can iterate on Doctor Sync and extend to Windows.

