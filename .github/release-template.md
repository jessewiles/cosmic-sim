## Installation

### Quick start (macOS & Linux)

1. Download the binary for your platform and `launch.sh` from the assets below.
2. Place both files in the same directory.
3. Open a terminal in that directory and run:

```bash
chmod +x launch.sh && ./launch.sh
```

The script auto-detects your OS and architecture, removes the macOS quarantine flag if needed, and launches the game.

---

### macOS (manual)

The binaries are unsigned. You must clear Apple's quarantine flag before running.

**Apple Silicon (M1/M2/M3):**
```bash
chmod +x cosmic-sim-macos-arm64
xattr -d com.apple.quarantine cosmic-sim-macos-arm64
./cosmic-sim-macos-arm64
```

**Intel:**
```bash
chmod +x cosmic-sim-macos-x86_64
xattr -d com.apple.quarantine cosmic-sim-macos-x86_64
./cosmic-sim-macos-x86_64
```

> Alternatively: right-click the binary in Finder → Open → Open anyway.

---

### Linux (manual)

```bash
chmod +x cosmic-sim-linux-x86_64
./cosmic-sim-linux-x86_64
```

---

### Windows

Download `cosmic-sim-windows-x86_64.exe` and double-click to run. If SmartScreen blocks it, click **More info → Run anyway**.

---

### API key

On first launch the game will prompt for your Anthropic API key and store it locally. You can also set it in your environment beforehand:

```bash
export ANTHROPIC_API_KEY=sk-...
```

## Changelog
