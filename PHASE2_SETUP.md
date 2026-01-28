# Phase 2 - Tauri Tray Integration Setup

## Current Status

Phase 2 structure is in place:
- ✅ Backend converted to library (`lib.rs`) with Tauri commands
- ✅ CLI binary still works (`main.rs`)
- ✅ Tauri app entry point (`tauri_main.rs`)
- ✅ System tray menu with JDK selection
- ✅ Basic frontend structure (tray-only, no window)

## Building & Running

### Prerequisites

```bash
# Install Node.js dependencies
npm install

# Ensure Rust toolchain is set up
rustup default stable
```

### Build CLI (standalone)

```bash
cd src-tauri
cargo build --release
# Binary: target/release/jdk-pulse
```

### Build Tauri App

```bash
# Install Tauri CLI globally (optional)
npm install -g @tauri-apps/cli

# Or use npx
npm run tauri build -- --features tauri
```

### Run Tauri App in Dev Mode

```bash
npm run tauri dev -- --features tauri
```

## Next Steps

1. **Add icons**: Create proper tray icons in `icons/` directory
2. **Test tray menu**: Verify JDK selection works
3. **Add tray icon badge**: Show active JDK version number in icon
4. **Polish UX**: Add settings window (Phase 3)

## Notes

- The app runs as a tray-only application (no main window)
- JDK selection updates `~/.jdk_current` immediately
- Tray menu refreshes after selection to show active JDK with ✓
