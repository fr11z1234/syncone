import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

interface SyncConfig {
  save_path: string | null;
  mods_path: string | null;
  cloud_path: string | null;
  supabase_url: string | null;
  supabase_key: string | null;
  bucket_name: string | null;
}

interface SyncStatus {
  save_local_mtime: number | null;
  save_cloud_mtime: number | null;
  mods_local_mtime: number | null;
  mods_cloud_mtime: number | null;
  save_local_newer: boolean;
  save_cloud_newer: boolean;
  mods_local_newer: boolean;
  mods_cloud_newer: boolean;
  save_path_used: string | null;
  mods_path_used: string | null;
}

function formatMtime(ts: number | null): string {
  if (ts == null) return "–";
  const d = new Date(ts * 1000);
  const day = String(d.getDate()).padStart(2, "0");
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const year = d.getFullYear();
  const h = String(d.getHours()).padStart(2, "0");
  const min = String(d.getMinutes()).padStart(2, "0");
  return `${day}-${month}-${year} ${h}:${min}`;
}

const savePathEl = document.querySelector("#save-path") as HTMLInputElement;
const modsPathEl = document.querySelector("#mods-path") as HTMLInputElement;
const cloudPathEl = document.querySelector("#cloud-path") as HTMLInputElement;
const supabaseUrlEl = document.querySelector("#supabase-url") as HTMLInputElement;
const supabaseKeyEl = document.querySelector("#supabase-key") as HTMLInputElement;
const bucketNameEl = document.querySelector("#bucket-name") as HTMLInputElement;
const saveConfigBtn = document.querySelector("#save-config");
const browseSaveBtn = document.querySelector("#browse-save");
const browseModsBtn = document.querySelector("#browse-mods");
const browseCloudBtn = document.querySelector("#browse-cloud");
const syncPullBtn = document.querySelector("#sync-pull");
const syncPushBtn = document.querySelector("#sync-push");
const syncStatusEl = document.querySelector("#sync-status") as HTMLElement;
const runAtStartupEl = document.querySelector("#run-at-startup") as HTMLInputElement;
const openStartupBtn = document.querySelector("#open-startup");
const refreshStatusBtn = document.querySelector("#refresh-status");
const saveLocalTimeEl = document.querySelector("#save-local-time") as HTMLElement;
const saveCloudTimeEl = document.querySelector("#save-cloud-time") as HTMLElement;
const modsLocalTimeEl = document.querySelector("#mods-local-time") as HTMLElement;
const modsCloudTimeEl = document.querySelector("#mods-cloud-time") as HTMLElement;
const saveBadgeEl = document.querySelector("#save-badge") as HTMLElement;
const modsBadgeEl = document.querySelector("#mods-badge") as HTMLElement;
const savePathUsedEl = document.querySelector("#save-path-used") as HTMLElement;
const modsPathUsedEl = document.querySelector("#mods-path-used") as HTMLElement;

function setStatus(text: string, isError = false) {
  if (!syncStatusEl) return;
  syncStatusEl.textContent = text;
  syncStatusEl.className = "status " + (isError ? "error" : "");
}

async function loadConfig() {
  try {
    const config = await invoke<SyncConfig>("get_config");
    savePathEl.value = config.save_path ?? "";
    modsPathEl.value = config.mods_path ?? "";
    cloudPathEl.value = config.cloud_path ?? "";
    supabaseUrlEl.value = config.supabase_url ?? "";
    supabaseKeyEl.value = config.supabase_key ?? "";
    bucketNameEl.value = config.bucket_name ?? "";
    const stored = localStorage.getItem("syncone_run_at_startup");
    runAtStartupEl.checked = stored === "true";
  } catch (e) {
    setStatus("Could not load settings: " + String(e), true);
  }
}

async function pickDirectory(currentPath: string): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    defaultPath: currentPath || undefined,
  });
  if (selected == null) return null;
  if (typeof selected === "string") return selected;
  return Array.isArray(selected) ? selected[0] ?? null : null;
}

function bindBrowse(button: Element | null, input: HTMLInputElement) {
  button?.addEventListener("click", async () => {
    const path = await pickDirectory(input.value);
    if (path) input.value = path;
  });
}

async function saveConfig() {
  const config: SyncConfig = {
    save_path: savePathEl.value.trim() || null,
    mods_path: modsPathEl.value.trim() || null,
    cloud_path: cloudPathEl.value.trim() || null,
    supabase_url: supabaseUrlEl.value.trim() || null,
    supabase_key: supabaseKeyEl.value.trim() || null,
    bucket_name: bucketNameEl.value.trim() || null,
  };
  try {
    await invoke("set_config", { config });
    setStatus("Paths saved.");
    await refreshSyncStatus();
  } catch (e) {
    setStatus("Could not save: " + String(e), true);
  }
}

function renderBadge(el: HTMLElement, localNewer: boolean, cloudNewer: boolean) {
  el.textContent = "";
  el.className = "status-badge";
  if (localNewer) {
    el.textContent = "Newer locally – ready to upload";
    el.classList.add("badge-push");
  } else if (cloudNewer) {
    el.textContent = "Newer in cloud – fetch";
    el.classList.add("badge-pull");
  }
}

function truncatePath(p: string, maxLen: number): string {
  if (p.length <= maxLen) return p;
  return "…" + p.slice(-maxLen + 1);
}

async function refreshSyncStatus() {
  if (!saveLocalTimeEl || !saveCloudTimeEl || !modsLocalTimeEl || !modsCloudTimeEl || !saveBadgeEl || !modsBadgeEl) return;
  try {
    const s = await invoke<SyncStatus>("get_sync_status");
    saveLocalTimeEl.textContent = formatMtime(s.save_local_mtime ?? null);
    saveCloudTimeEl.textContent = formatMtime(s.save_cloud_mtime ?? null);
    modsLocalTimeEl.textContent = formatMtime(s.mods_local_mtime ?? null);
    modsCloudTimeEl.textContent = formatMtime(s.mods_cloud_mtime ?? null);
    if (savePathUsedEl) {
      savePathUsedEl.textContent = s.save_path_used ? truncatePath(s.save_path_used, 55) : "";
      savePathUsedEl.title = s.save_path_used ?? "";
    }
    if (modsPathUsedEl) {
      modsPathUsedEl.textContent = s.mods_path_used ? truncatePath(s.mods_path_used, 55) : "";
      modsPathUsedEl.title = s.mods_path_used ?? "";
    }
    renderBadge(saveBadgeEl, s.save_local_newer, s.save_cloud_newer);
    renderBadge(modsBadgeEl, s.mods_local_newer, s.mods_cloud_newer);
  } catch {
    saveLocalTimeEl.textContent = "–";
    saveCloudTimeEl.textContent = "–";
    modsLocalTimeEl.textContent = "–";
    modsCloudTimeEl.textContent = "–";
    if (savePathUsedEl) savePathUsedEl.textContent = "";
    if (modsPathUsedEl) modsPathUsedEl.textContent = "";
    saveBadgeEl.textContent = "";
    saveBadgeEl.className = "status-badge";
    modsBadgeEl.textContent = "";
    modsBadgeEl.className = "status-badge";
  }
}

type SyncTarget = "save" | "mods" | "both";

const modalOverlay = document.getElementById("modal-overlay") as HTMLElement;
const modalBody = document.getElementById("modal-body") as HTMLElement;
const modalCancel = document.getElementById("modal-cancel") as HTMLElement;
const modalForce = document.getElementById("modal-force") as HTMLElement;

function showModal(body: string, confirmLabel = "Upload anyway"): Promise<boolean> {
  return new Promise((resolve) => {
    modalBody.textContent = body;
    modalForce.textContent = confirmLabel;
    modalOverlay.classList.remove("hidden");

    function cleanup() {
      modalOverlay.classList.add("hidden");
      modalCancel.removeEventListener("click", onCancel);
      modalForce.removeEventListener("click", onForce);
    }
    function onCancel() { cleanup(); resolve(false); }
    function onForce() { cleanup(); resolve(true); }

    modalCancel.addEventListener("click", onCancel);
    modalForce.addEventListener("click", onForce);
  });
}

async function doSyncPull(target: SyncTarget = "both", force = false, successMessage?: string) {
  setStatus(target === "both" ? "Fetching..." : `Fetching ${target}...`);
  try {
    const result = await invoke<{ ok: boolean; message: string }>("do_sync_pull", {
      target: target === "both" ? undefined : target,
      force: force || undefined,
    });
    setStatus(successMessage && result.ok ? successMessage : result.message);
    await refreshSyncStatus();
  } catch (e) {
    const msg = String(e);
    if (msg.includes("PROGRESS_WARNING:")) {
      const warningText = msg.replace("PROGRESS_WARNING:", "").trim();
      setStatus("");
      const confirmed = await showModal(warningText, "Fetch anyway");
      if (confirmed) {
        await doSyncPull(target, true);
      } else {
        setStatus("Fetch cancelled.");
      }
    } else {
      setStatus("Error: " + msg, true);
    }
  }
}

async function doSyncPush(target: SyncTarget = "both", force = false) {
  setStatus(target === "both" ? "Uploading..." : `Uploading ${target}...`);
  try {
    const result = await invoke<{ ok: boolean; message: string }>("do_sync_push", {
      target: target === "both" ? undefined : target,
      force: force || undefined,
    });
    setStatus(result.message);
    await refreshSyncStatus();
  } catch (e) {
    const msg = String(e);
    if (msg.includes("PROGRESS_WARNING:")) {
      const warningText = msg.replace("PROGRESS_WARNING:", "").trim();
      setStatus("");
      const confirmed = await showModal(warningText);
      if (confirmed) {
        await doSyncPush(target, true);
      } else {
        setStatus("Upload cancelled.");
      }
    } else {
      setStatus("Error: " + msg, true);
    }
  }
}

const UPDATE_CHECK_TIMEOUT_MS = 5_000;

async function checkForAppUpdate() {
  try {
    const update = await Promise.race([
      check(),
      new Promise<null>((_, reject) =>
        setTimeout(() => reject(new Error("timeout")), UPDATE_CHECK_TIMEOUT_MS)
      ),
    ]);
    if (update) {
      const confirmed = await showModal(
        `A new version of SyncONE is available (${update.version}).\n\nClick "Update now" to download and install it. The app will restart automatically.`,
        "Update now"
      );
      if (confirmed) {
        setStatus("Downloading update...");
        await update.downloadAndInstall();
        await relaunch();
      }
    }
  } catch {
    // Silently ignore update check failures (e.g. no internet, timeout)
  }
}

function initStartupToggle() {
  runAtStartupEl?.addEventListener("change", () => {
    localStorage.setItem("syncone_run_at_startup", String(runAtStartupEl.checked));
    setStatus(
      runAtStartupEl.checked
        ? "Remember to save paths. On next boot SyncONE will run at startup (if you added it to Startup)."
        : "SyncONE will no longer run at startup."
    );
  });
}

window.addEventListener("DOMContentLoaded", async () => {
  const versionEl = document.querySelector("#app-version");
  if (versionEl) {
    try {
      const v = await getVersion();
      versionEl.textContent = "v" + v;
    } catch {
      versionEl.textContent = "v?";
    }
  }
  // Run update check in background so app loads immediately; modal appears when check completes
  void checkForAppUpdate();

  await loadConfig();
  bindBrowse(browseSaveBtn, savePathEl);
  bindBrowse(browseModsBtn, modsPathEl);
  bindBrowse(browseCloudBtn, cloudPathEl);
  saveConfigBtn?.addEventListener("click", saveConfig);
  syncPullBtn?.addEventListener("click", () => doSyncPull("both"));
  syncPushBtn?.addEventListener("click", () => doSyncPush("both"));

  document.getElementById("sync-status-cards")?.addEventListener("click", (e) => {
    const btn = (e.target as HTMLElement).closest("button[data-target]");
    if (!btn) return;
    const target = (btn as HTMLButtonElement).dataset.target as SyncTarget;
    if ((btn as HTMLButtonElement).classList.contains("card-fetch")) {
      void doSyncPull(target);
    } else if ((btn as HTMLButtonElement).classList.contains("card-upload")) {
      void doSyncPush(target);
    }
  });

  initStartupToggle();
  openStartupBtn?.addEventListener("click", async () => {
    try {
      await invoke("open_startup_folder");
    } catch (e) {
      setStatus("Could not open folder: " + String(e), true);
    }
  });

  refreshStatusBtn?.addEventListener("click", refreshSyncStatus);
  await refreshSyncStatus();

  // Refresh status every 30 seconds so local folder changes (e.g. after saving in-game) are picked up
  const REFRESH_INTERVAL_MS = 30_000;
  setInterval(() => void refreshSyncStatus(), REFRESH_INTERVAL_MS);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") void refreshSyncStatus();
  });
  window.addEventListener("focus", () => void refreshSyncStatus());

  const hasCloud =
    (supabaseUrlEl.value.trim() && supabaseKeyEl.value.trim() && bucketNameEl.value.trim()) ||
    cloudPathEl.value.trim();
  if (savePathEl.value.trim() && modsPathEl.value.trim() && hasCloud) {
    setStatus("Checking for updates...");
    await doSyncPull("both", false, "Auto pulled newest version on startup");
  }
});
