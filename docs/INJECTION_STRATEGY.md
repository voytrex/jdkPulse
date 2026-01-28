## JDK-Pulse – Injection Strategy

This document describes **how** JDK‑Pulse propagates the selected JDK into:
- New and existing **terminal sessions**
- **IDEs** and their embedded terminals
- **AI agents** (Cursor, Copilot, etc.)

The guiding principles:
- **Source of truth**: A single, canonical definition of “current JDK” per user.
- **Non-invasive**: Never fully own the shell config; only manage small, clearly scoped blocks.
- **Observable**: Easy to verify and debug when something goes wrong (“Doctor Sync”).

---

### 1. Global State: The Canonical JDK

JDK‑Pulse maintains a canonical state file, e.g.:

- macOS / Linux: `~/.jdk_current`
- Windows: `%USERPROFILE%\.jdk_current`

Contents (example):

```text
/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home
```

The Orchestrator always writes this file when a JDK is selected. All shell hooks and helpers **read from this file**, not from app‑internal state, so that even if the app is not running, state remains inspectable.

---

### 2. Unix-like Systems (macOS, Linux)

#### 2.1 New Sessions – Shell Init Integration

For new terminals, we integrate with the user’s login shell config.

Target files:
- `~/.zshrc` (default on modern macOS)
- `~/.bashrc` / `~/.bash_profile` where appropriate

JDK‑Pulse appends a **tagged block**:

```bash
# >>> JDK-Pulse shell hook >>>
_jdk_pulse_apply() {
  local _state_file="$HOME/.jdk_current"
  if [ -f "$_state_file" ]; then
    local new_home
    new_home="$(cat "$_state_file" 2>/dev/null)"
    if [ -n "$new_home" ] && [ -d "$new_home" ] && [ "$JAVA_HOME" != "$new_home" ]; then
      export JAVA_HOME="$new_home"
      case ":$PATH:" in
        *":$JAVA_HOME/bin:"*) ;; # already present
        *) export PATH="$JAVA_HOME/bin:$PATH" ;;
      esac
    fi
  fi
}

precmd_functions=(${precmd_functions[@]} _jdk_pulse_apply)
# <<< JDK-Pulse shell hook <<<
```

Key details:
- Uses `precmd` (zsh) or `PROMPT_COMMAND`/`precmd` equivalent in bash to run **before every prompt**, i.e. every time the user hits Enter.
- Reads from `~/.jdk_current` and updates `JAVA_HOME` and the `PATH` if necessary.
- Avoids duplicating `JAVA_HOME/bin` on the `PATH`.
- Wrapped in clearly delimited comments so it can be programmatically removed.

For `bash`, the hook can be adapted to use `PROMPT_COMMAND`:

```bash
_jdk_pulse_precmd() {
  _jdk_pulse_apply
}
if [[ "$PROMPT_COMMAND" != *"_jdk_pulse_precmd"* ]]; then
  PROMPT_COMMAND="_jdk_pulse_precmd; $PROMPT_COMMAND"
fi
```

#### 2.2 Active Sessions – “Hot Reload”

Because the hook runs on each prompt, **already-open terminals** will pick up a JDK change as soon as:
- The user presses Enter (i.e. a new command is issued), or
- The shell re-renders the prompt.

This achieves “session injection” without needing to attach to or restart the terminal process.

#### 2.3 IDE Terminals

Most IDE embedded terminals (IntelliJ, VS Code, Cursor) are just shells with the same config files:
- When JDK‑Pulse installs its shell hook, IDE terminals behave like regular shells.
- The next prompt in those embedded terminals will sync with `~/.jdk_current`.

Where IDEs allow custom shell command or environment:
- JDK‑Pulse can provide optional **snippets** or **profile templates** for users to copy/paste, but the core strategy remains the same: read from `~/.jdk_current`.

---

### 3. Windows Strategy

Windows requires a different approach because shells and IDEs often read environment variables at process startup.

#### 3.1 Canonical State File

JDK‑Pulse still maintains `%USERPROFILE%\.jdk_current` as the single source of truth. Any PowerShell / CMD integration should read from this file where possible.

#### 3.2 System / User Environment Variables

To influence tools that rely on the Windows environment:

1. **Update Environment Variables**
   - Update `JAVA_HOME` for the **User** or **System** scope (configurable).
   - Optionally adjust `PATH`:
     - Prepend `%JAVA_HOME%\bin` if not already present.

2. **Broadcast Changes**
   - After updating, send `WM_SETTINGCHANGE` (`"Environment"`) to inform:
     - Windows Explorer
     - Some IDEs / processes that listen for environment changes.

3. **Limitations**
   - Already-running processes that do not re-read the environment may still require restart.
   - JDK‑Pulse can surface this in the UI (e.g. “Restart IntelliJ to apply system JDK changes.”).

#### 3.3 Shell Integration (PowerShell / CMD)

For better real-time sync, JDK‑Pulse can optionally install shell snippets:

- **PowerShell profile** (e.g. `Documents\PowerShell\Microsoft.PowerShell_profile.ps1`):

```powershell
function Invoke-JdkPulseSync {
  $stateFile = Join-Path $env:USERPROFILE ".jdk_current"
  if (Test-Path $stateFile) {
    $newHome = Get-Content $stateFile -ErrorAction SilentlyContinue
    if ($null -ne $newHome -and (Test-Path $newHome) -and $env:JAVA_HOME -ne $newHome) {
      $env:JAVA_HOME = $newHome
      if (-not ($env:PATH -like "*$env:JAVA_HOME\bin*")) {
        $env:PATH = "$env:JAVA_HOME\bin;$env:PATH"
      }
    }
  }
}

Register-EngineEvent PowerShell.OnIdle -Action { Invoke-JdkPulseSync } | Out-Null
```

- **CMD** (optional and more limited):
  - Provide a `jdkpulse.cmd` helper that reads `.jdk_current` and sets `JAVA_HOME` / `PATH` for that session.

---

### 4. AI Agent & IDE Context Sync

The goal is that when an AI agent runs `mvn test` or `./mvnw compile`, it knows which JDK to assume.

#### 4.1 Cursor / Editor AI Hints

JDK‑Pulse can generate **hint files** in the project or user space, e.g.:

- `.cursor/rules/JAVA_HOME.md` or similar:

```markdown
## Java Environment

- Active JDK: 21
- JAVA_HOME: /Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home
- PATH includes: $JAVA_HOME/bin

When running Java-related commands (maven, gradle, java, javac, testcontainers), always assume this JAVA_HOME value unless the user explicitly overrides it.
```

These files can be updated whenever the active JDK changes or when the user focuses a particular project (for per‑project overrides).

#### 4.2 IDE Settings (Optional Deep Integration)

Where APIs allow, JDK‑Pulse can:
- Generate or update per‑IDE config files:
  - IntelliJ `.idea` JDK settings (requires careful handling; likely a later milestone).
  - VS Code workspace settings (`"java.jdt.ls.java.home"`).
- Alternatively, provide **one-click copy** snippets in the UI that users paste into their IDE settings.

---

### 5. Doctor Sync

“Doctor Sync” verifies alignment across several layers:

- **State File vs System**
  - `~/.jdk_current` versus what `java -version` reports in:
    - A spawned non-interactive shell using the configured profile.

- **State File vs Shell Environment**
  - Check `$JAVA_HOME` in a spawned interactive shell with the JDK‑Pulse hook enabled.

- **Docker / Testcontainers (Future)**
  - Validate Docker availability (`docker info`).
  - Inspect containers where relevant to ensure the expected JDK is used (for advanced use cases).

Output is a structured report (JSON) that the UI can render as:
- ✅ / ⚠️ / ❌ indicators for each layer.

---

### 6. Safety & Uninstallation

- All injected shell code is:
  - Clearly delimited with comments.
  - Idempotent (safe to apply multiple times).
  - Removable via a single **“Remove Shell Integration”** action.

- No destructive operations:
  - Never delete user config files.
  - Only modify within managed blocks.

This strategy gives JDK‑Pulse the ability to:
- **Instantly reflect** JDK changes across active sessions.
- Provide **strong guarantees** for tools and AI agents.
- Remain **transparent and reversible** for the user.

