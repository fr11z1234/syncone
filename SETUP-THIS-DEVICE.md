# SyncONE – Setup on This Device

Use this checklist to get the project building and running on a new machine (e.g. desktop after developing on a laptop).

## Already installed on this device

- **Node.js** v20.11.1  
- **npm** 10.2.4  

## 1. Install Rust (required for Tauri)

Rust is needed to build the desktop app backend.

1. Open: **https://rustup.rs**
2. Download and run **rustup-init.exe** (Windows).
3. In the installer, choose the default option (press Enter).
4. **Restart your terminal** (or Cursor) so `rustc` and `cargo` are on your PATH.
5. Verify:
   ```powershell
   rustc --version
   cargo --version
   ```

## 2. Windows: Visual Studio Build Tools (C++)

On Windows, the Rust toolchain needs the **Microsoft C++ build tools** to compile native code.

**Option A – Build Tools only (smaller install)**  
1. Download: **https://visualstudio.microsoft.com/visual-cpp-build-tools/**  
2. Run the installer and select the workload: **“Desktop development with C++”**.  
3. Install and restart if prompted.

**Option B – Full Visual Studio**  
If you already use Visual Studio, install the workload **“Desktop development with C++”** in the Visual Studio Installer.

## 3. Install project dependencies

In the project folder (`syncone`), run:

```powershell
cd "c:\Users\Mads\Desktop\schedule1\Syncone Source\syncone"
npm install
```

(This was run for you during setup; re-run if you pull changes that touch `package.json`.)

## 4. Run the app

```powershell
npm run tauri dev
```

To build an installable `.exe`:

```powershell
npm run tauri build
```

Output is in `src-tauri/target/release/` and under `src-tauri/target/release/bundle/` for the installer.

---

**Summary:** Install **Rust** (rustup) and **Visual Studio Build Tools (C++)** on this device, then use `npm install` and `npm run tauri dev` in the `syncone` folder.
