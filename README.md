# SyncONE

Desktop app that syncs **Schedule I** saves and mods to a shared cloud (e.g. Supabase Storage or a folder synced with Google Drive), so your group always has the latest save without depending on the lobby host.

## How it works

1. **Set paths** in the app:
   - **Save folder**: your Schedule I save folder (e.g. `C:\Users\...\AppData\LocalLow\TVGS\Schedule I\Saves\<ID>`)
   - **Mods folder**: Schedule I mods (e.g. `C:\Program Files (x86)\Steam\steamapps\common\Schedule I\mods`)
   - **Cloud**: either **Supabase** (recommended – one bucket, `Save.zip` and `Mods.zip`) or a **cloud folder** that syncs with Google Drive / OneDrive

2. **On startup**: Open SyncONE → it automatically fetches the latest save/mods from the cloud if a newer version exists.

3. **When you’re done playing**: Click **Upload to cloud** (or upload Save/Mods individually from each card) so others can get the latest when they start.

You can sync **Save** and **Mods** separately or both at once.

## Requirements

- **Node.js** and **npm** (to build the frontend)
- **Rust** (to build the Tauri app): [rustup.rs](https://rustup.rs)
- **Windows**: Visual Studio Build Tools (C++ workload) or “Desktop development with C++”

## Build and run

```bash
npm install
npm run tauri dev
```

To build an installable .exe:

```bash
npm run tauri build
```

Output is in `src-tauri/target/release/` (`.exe` and installer under `bundle/`).

## Run at Windows startup

1. Check “Run SyncONE at Windows startup” in the app.
2. Click **Open Startup folder**.
3. Create a shortcut to `Syncone.exe` (from `src-tauri/target/release/` or from the installer) and put it in the opened Startup folder.

SyncONE will then automatically fetch the latest save when the PC starts (once your cloud/Supabase is available).

## Non-hosts stuck on "Syncing" after pulling the save?

If people who **join** get stuck on "Syncing" the first time they launch after a pull, see **[TROUBLESHOOTING.md](TROUBLESHOOTING.md)**. SyncONE now injects the usual fix (`HasExitedRV.json`) into the save after every pull to reduce this.

## Technical

- **Tauri 2** (Rust + web UI)
- Config is stored in `%APPDATA%\Syncone\syncone_config.json`
- Sync is based on “last modified” (mtime): newer files in the cloud overwrite local, and vice versa on upload.
