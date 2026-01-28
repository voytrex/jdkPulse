## JDK-Pulse

JDK‑Pulse is a cross‑platform menu bar / taskbar app that keeps your **system JDK**, **terminals**, **IDEs**, and **AI agents** aligned on the same Java version with a single click.

### What it does

- **Visual JDK status** in the tray (e.g. `8`, `17`, `21`, `25`).
- **One-click switching** of the active JDK across:
  - `JAVA_HOME` and `PATH`
  - New and existing terminal sessions
  - IDE‑embedded terminals
- Planned **“Doctor Sync”** to verify that `java -version`, `JAVA_HOME`, Docker/Testcontainers, and AI tools all agree on the same JDK.

### Documentation

- `docs/ARCHITECTURE.md` – high-level system architecture.
- `docs/INJECTION_STRATEGY.md` – how JDK‑Pulse injects the selected JDK into shells, IDEs, and agents.
- `docs/SPECIFICATION.md` – technical spec, API surface, and implementation roadmap.

### Roadmap (high level)

1. **Milestone 1 – macOS preview**
   - Tauri-based tray app
   - macOS JDK detection via `java_home -V`
   - Shell integration for `zsh` / `bash`
2. **Milestone 2 – Windows support**
3. **Milestone 3 – Docker/Testcontainers awareness**
4. **Milestone 4 – AI & IDE context sync**

