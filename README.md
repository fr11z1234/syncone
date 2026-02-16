# Syncone

Desktop-app der synkroniserer **Schedule I** saves og mods til en fælles cloud-mappe (fx Google Drive), så I ikke behøver at vente på lobby-ejeren for at få det seneste save.

## Sådan virker det

1. **Vælg stier** i appen:
   - **Save-mappe**: den konkrete save-mappe (fx `C:\Users\...\AppData\LocalLow\TVGS\Schedule I\Saves\<ID>`)
   - **Mods-mappe**: Schedule I mods (fx `C:\Program Files (x86)\Steam\steamapps\common\Schedule I\mods`)
   - **Cloud-mappe**: en mappe der synces med Google Drive (eller anden cloud) – alle i gruppen bruger den samme mappe

2. **Ved opstart**: Åbn Syncone → den henter automatisk nyeste save/mod fra cloud, hvis der er en nyere version.

3. **Efter I er færdige med at spille**: Klik **Upload til cloud** → så kan de andre hente det seneste når de starter.

I cloud-mappen opretter appen to undermapper: `Save` og `Mods`. Kun disse synces; resten af drevet påvirkes ikke.

## Krav

- **Node.js** og **npm** (til at bygge frontend)
- **Rust** (til at bygge Tauri-appen): [rustup.rs](https://rustup.rs)
- **Windows**: Visual Studio Build Tools (C++ workload) eller “Desktop development with C++”

## Byg og kør

```bash
npm install
npm run tauri dev
```

Til at lave en installerbar .exe:

```bash
npm run tauri build
```

Output ligger i `src-tauri/target/release/` (`.exe` og evt. installer under `bundle/`).

## Kør ved Windows-opstart

1. Sæt flueben ved “Kør Syncone ved Windows-opstart”.
2. Klik **Åbn Opstart-mappe**.
3. Opret et genvej til `Syncone.exe` (fra `src-tauri/target/release/` eller fra installeren) og læg det i den åbnede Opstart-mappe.

Så henter Syncone automatisk det nyeste save når PC’en starter (hvis Google Drive/cloud allerede er synced).

## Teknisk

- **Tauri 2** (Rust + web UI)
- Config gemmes i `%APPDATA%\Syncone\syncone_config.json`
- Sync baserer sig på “senest ændret” (mtime): nyere filer i cloud overskriver lokale og omvendt ved upload.
