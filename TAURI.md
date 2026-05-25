# SillyTauri

SillyTavern wrapped as a native Tauri application for Desktop (Windows, macOS, Linux) and Android.

## Architecture

This app uses [Tauri v2](https://v2.tauri.app/) to wrap the SillyTavern Node.js server and frontend into a native application:

- **Desktop**: The Tauri app spawns a Node.js process running the SillyTavern server, then displays the UI in a native webview window.
- **Android**: Uses the same Tauri v2 mobile support with the Node.js server running as a background process.

## Prerequisites

### All Platforms
- [Node.js](https://nodejs.org/) >= 20
- [Rust](https://rustup.rs/) (latest stable)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/): `cargo install tauri-cli --version "^2"`

### Desktop (Linux)
```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

### Desktop (macOS)
- Xcode Command Line Tools

### Desktop (Windows)
- [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on Windows 10+)

### Android
- [Android Studio](https://developer.android.com/studio)
- Android SDK & NDK
- Set `ANDROID_HOME` and `NDK_HOME` environment variables

## Setup

```bash
# Install Node.js dependencies
npm install

# Initialize Android (if targeting Android)
npm run tauri:android:init
```

## Development

```bash
# Desktop development
npm run tauri:dev

# Android development (requires emulator or device)
npm run tauri:android:dev
```

## Building

```bash
# Desktop build (produces installer for current platform)
npm run tauri:build

# Android build (produces APK/AAB)
npm run tauri:android:build
```

## Data Import

The app supports importing user data from a previous SillyTavern installation:

1. Create a `.zip` file of your previous installation's `/data` directory
2. In the app, use the "Import Data from Previous Install" button on the loading screen
3. Or after the app loads, call the import via the Tauri API:
   ```javascript
   await window.__TAURI__.core.invoke('pick_and_import_data');
   ```

The imported data will be extracted to the app's data directory, preserving the folder structure.

## Project Structure

```
├── src-tauri/           # Tauri application (Rust backend)
│   ├── src/
│   │   ├── lib.rs       # Main app logic, commands, setup
│   │   ├── main.rs      # Desktop entry point
│   │   └── server.rs    # Node.js server lifecycle management
│   ├── capabilities/    # Tauri permission capabilities
│   ├── dist/            # Loading page (shown while server starts)
│   ├── icons/           # App icons
│   ├── Cargo.toml       # Rust dependencies
│   └── tauri.conf.json  # Tauri configuration
├── server.js            # SillyTavern server entry point
├── src/                 # SillyTavern server source
└── public/              # SillyTavern frontend
```
