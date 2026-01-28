## JDK-Pulse – System Architecture

### Overview

JDK‑Pulse is a cross‑platform menu bar / taskbar application focused on eliminating “context drift” between:
- **System JDK** (`JAVA_HOME`, `PATH`)
- **Developer tools** (Maven, Gradle, Testcontainers, Docker)
- **IDEs & AI agents** (Cursor, VS Code, IntelliJ, Copilot, etc.)

It provides **one-click, visually obvious** JDK selection and ensures that the chosen version is consistently applied across shells, tools, and agents.

---

### High-Level Architecture

- **Frontend (UI Layer)**
  - Built with **Tauri v2 + React** (or Svelte/Vanilla as an implementation detail).
  - Renders a **menu bar / tray icon** showing the currently active JDK (e.g. `8`, `17`, `21`, `25`).
  - Provides menus for:
    - Switching the active JDK
    - Running “Doctor Sync” checks
    - Opening settings / logs.

- **Backend Core (Rust / Tauri Commands)**
  - Implemented in Rust as Tauri commands and background tasks.
  - Responsibilities:
    - Discover installed JDKs on each platform.
    - Maintain the **active JDK selection** and persist it.
    - Perform **system updates** (JAVA_HOME, PATH, registry / environment changes).
    - Coordinate **session injection** for shells and IDEs.
    - Run “Doctor Sync” diagnostics.

- **Platform Adapters**
  - Encapsulate OS‑specific details:
    - Path discovery for JDKs.
    - Environment variable manipulation.
    - Registry / broadcast messaging on Windows.
    - Shell hook file locations on macOS / Linux.

- **State & Persistence**
  - Canonical “current JDK” stored in:
    - **State file**: `~/.jdk_current` (or platform‑appropriate equivalent).
    - **App config** (Tauri/Rust side) for additional metadata (display name, vendor, etc.).
  - Optional project‑level hints:
    - `.jdkpulse.json` or existing build files (`pom.xml`, `build.gradle`, etc.) to drive auto‑switching.

---

### Core Components

#### 1. Watcher (JDK Discovery Service)

**Responsibility**: Detect and index available JDK installations.

- **macOS**
  - Uses `/usr/libexec/java_home -V` as primary mechanism.
  - Optionally scans `/Library/Java/JavaVirtualMachines` for additional metadata.
- **Windows**
  - Scans common locations, e.g.:
    - `C:\Program Files\Java\`
    - `C:\Program Files\Eclipse Adoptium\`
  - Reads relevant registry keys (e.g. `HKLM\SOFTWARE\JavaSoft\JDK`).
- **Linux**
  - Scans:
    - `/usr/lib/jvm/`
    - `update-alternatives --display java` where available.

The Watcher maintains a list like:

- **ID**: internal identifier (e.g. `temurin-21`, `oracle-8u322`)
- **Version**: semantic version (`8`, `11`, `17`, `21`, `25`, etc.)
- **Home Path**: full `JAVA_HOME`
- **Vendor / Distribution**: (Temurin, Oracle, Zulu, etc.)

This list is exposed to the UI for selection and to the Orchestrator for activation.

---

#### 2. Tray Manager (System Tray / Menu Bar UI)

**Responsibility**: Present the active JDK and handle user interactions.

- Shows an **icon badge** (e.g. “21”) representing the current JDK major version.
- Dropdown menu:
  - **Current JDK** (highlighted)
  - **Available JDKs** (radio selection)
  - Separator
  - **Doctor Sync…**
  - **Preferences…**
  - **Quit**

It talks to the backend via Tauri commands:

- `list_jdks() -> [JdkInfo]`
- `get_active_jdk() -> JdkInfo`
- `set_active_jdk(id: String)`
- `run_doctor_sync() -> DoctorReport`

---

#### 3. Orchestrator (Switching & Sync Engine)

**Responsibility**: When a user (or an auto‑switch rule) selects a JDK, the Orchestrator ensures that **all relevant environments converge** on that JDK.

Flow when user picks a JDK:

1. **Update Canonical State**
   - Write `JAVA_HOME` value into the global state file, e.g.:
     - macOS/Linux: `~/.jdk_current`
     - Windows: `%USERPROFILE%\.jdk_current`
   - Update persisted config.

2. **System Update**
   - On macOS/Linux:
     - Optionally update shell config snippets managed by JDK‑Pulse (not override user custom logic).
   - On Windows:
     - Update `JAVA_HOME` in the registry and adjust the system/user `PATH` (if configured).
     - Broadcast environment change (`WM_SETTINGCHANGE`).

3. **Session Injection**
   - Trigger mechanisms that cause **already-open terminals and IDE terminals** to pick up the new value (see `INJECTION_STRATEGY.md`).
   - Optionally emit events/logs to be surfaced in the UI.

4. **AI Agent Sync**
   - Generate/update **agent hint files**:
     - `.cursorrules` / `.cursor/rules/JAVA_HOME.md`
     - `.editorconfig` snippets or workspace settings overrides for IDEs.

---

### Cross‑Cutting Concerns

- **Security / Safety**
  - Do not blindly overwrite user shell configs; instead:
    - Append clearly delimited, comment‑tagged blocks (e.g. `# >>> JDK‑Pulse >>>` … `# <<< JDK‑Pulse <<<`).
    - Provide a “Remove Integration” button that uninstalls hooks cleanly.

- **Telemetry & Logs (Optional)**
  - Local logs for debugging why a sync failed:
    - JDK detection
    - Shell hook integration
    - Windows registry updates

- **Extensibility**
  - Design Orchestrator to support **other runtimes**:
    - Node.js (`nvm`, `fnm`‑like behavior)
    - Maven/Gradle wrappers
    - Kotlin/Scala toolchains.

---

### Initial Milestone Scope

**Milestone 1 – macOS Preview (Single User JDK Sync)**

- Tauri app with:
  - Menu bar icon showing current JDK major version.
  - Dropdown list of installed JDKs from `java_home -V`.
- When a JDK is selected:
  - Writes to `~/.jdk_current`.
  - Sets up / updates a shell hook snippet for `zsh` (and optionally `bash`).
- Provides a basic “Doctor Sync” that:
  - Compares `java -version` vs `echo $JAVA_HOME`.
  - Verifies that the currently active shell session is aligned with `~/.jdk_current`.

Later milestones will extend to Windows/Linux, deeper Docker/Testcontainers checks, and richer AI‑agent integration.

