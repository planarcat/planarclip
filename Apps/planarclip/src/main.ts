import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ── DOM refs ────────────────────────────────────────────────────────────

const statusDot = document.getElementById("statusDot")!;
const statusText = document.getElementById("statusText")!;
const lastSync = document.getElementById("lastSync")!;
const pairInput = document.getElementById("pairInput") as HTMLInputElement;
const pairBtn = document.getElementById("pairBtn") as HTMLButtonElement;
const myCodeEl = document.getElementById("myCode")!;
const myCodeSection = document.getElementById("myCodeSection")!;
const lanList = document.getElementById("lanList")!;

// Overlays
const codeOverlay = document.getElementById("codeOverlay")!;
const codeInput = document.getElementById("codeInput") as HTMLInputElement;
const codeSubmit = document.getElementById("codeSubmit") as HTMLButtonElement;
const codeCancel = document.getElementById("codeCancel") as HTMLButtonElement;
const codeError = document.getElementById("codeError")!;

const requestOverlay = document.getElementById("requestOverlay")!;
const requestFrom = document.getElementById("requestFrom")!;
const requestCode = document.getElementById("requestCode")!;
const requestReject = document.getElementById("requestReject") as HTMLButtonElement;

// ── Status helpers ──────────────────────────────────────────────────────

function setStatus(state: "offline" | "connecting" | "online") {
  statusDot.className = `status-dot ${state}`;
  const labels: Record<string, string> = {
    offline: "Offline",
    connecting: "Connecting...",
    online: "Connected",
  };
  statusText.textContent = labels[state];
}

function formatTime() {
  return new Date().toLocaleTimeString();
}

// ── Background polling ──────────────────────────────────────────────────

async function refreshStatus() {
  try {
    const status = await invoke<string>("get_status");
    if (status === "connected") {
      setStatus("online");
    } else {
      setStatus("offline");
    }
  } catch (e) {
    console.error("Status check failed:", e);
  }
}

async function loadPairingCode() {
  try {
    const code = await invoke<string>("get_pairing_code");
    myCodeEl.textContent = code;
    myCodeSection.style.display = "block";
  } catch (e) {
    console.error("Failed to get pairing code:", e);
  }
}

// ── Existing pair command (remote / signalling) ─────────────────────────

pairBtn.addEventListener("click", async () => {
  const code = pairInput.value.trim();
  if (code.length !== 6 || !/^\d+$/.test(code)) {
    lastSync.textContent = "Please enter a valid 6-digit code";
    return;
  }
  lastSync.textContent = `Pairing with ${code}...`;
  setStatus("connecting");
  pairBtn.disabled = true;

  try {
    await invoke<string>("pair", { code });
    lastSync.textContent = `Paired — ${formatTime()}`;
    setStatus("online");
  } catch (e) {
    lastSync.textContent = `Pairing failed: ${e}`;
    setStatus("offline");
  } finally {
    pairBtn.disabled = false;
  }
});

// ── LAN device list ─────────────────────────────────────────────────────

async function loadLanDevices() {
  try {
    const devices = await invoke<Array<{ name: string; peer_id: string; ip: string; port: number }>>("get_lan_devices");
    renderLanDevices(devices);
  } catch (e) {
    console.error("Failed to load LAN devices:", e);
  }
}

function renderLanDevices(
  devices: Array<{ name: string; peer_id: string; ip: string; port: number }>
) {
  if (devices.length === 0) {
    lanList.innerHTML = '<div class="empty-hint">No devices found</div>';
    return;
  }
  lanList.innerHTML = devices
    .map(
      (d) => `
    <div class="lan-item">
      <div>
        <div class="dev-name">${esc(d.name)}</div>
        <div class="dev-ip">${esc(d.ip)}:${d.port}</div>
      </div>
      <button class="lan-connect" data-ip="${esc(d.ip)}" data-port="${d.port}">Connect</button>
    </div>`
    )
    .join("");

  lanList.querySelectorAll(".lan-connect").forEach((btn) => {
    btn.addEventListener("click", async (e) => {
      const ip = (e.target as HTMLElement).dataset.ip!;
      const port = parseInt((e.target as HTMLElement).dataset.port!);
      await startLanConnect(ip, port);
    });
  });
}

// ── Initiator: connect to LAN device ────────────────────────────────────

async function startLanConnect(ip: string, port: number) {
  lastSync.textContent = `Connecting to ${ip}...`;
  setStatus("connecting");

  try {
    const result = await invoke<string>("connect_lan", { ip, port });
    if (result === "awaiting_code") {
      showCodeOverlay();
    }
    // "connected" is handled by the connection-established event
  } catch (e) {
    lastSync.textContent = `Connection failed: ${e}`;
    setStatus("offline");
  }
}

// ── Initiator: pairing code overlay ─────────────────────────────────────

function showCodeOverlay() {
  codeOverlay.classList.remove("hidden");
  codeInput.value = "";
  codeError.textContent = "";
  codeInput.focus();
}

function hideCodeOverlay() {
  codeOverlay.classList.add("hidden");
}

codeSubmit.addEventListener("click", async () => {
  const code = codeInput.value.trim();
  if (code.length !== 6 || !/^\d+$/.test(code)) {
    codeError.textContent = "Please enter a 6-digit code";
    return;
  }
  codeSubmit.disabled = true;
  codeError.textContent = "";

  try {
    await invoke<string>("submit_pairing_code", { code });
    hideCodeOverlay();
    // connection-established event will update status
  } catch (e) {
    codeError.textContent = `${e}`;
  } finally {
    codeSubmit.disabled = false;
  }
});

codeCancel.addEventListener("click", () => {
  hideCodeOverlay();
  setStatus("offline");
});

// ── Responder: connection request overlay ───────────────────────────────

requestReject.addEventListener("click", async () => {
  try {
    await invoke("reject_connection");
  } catch (e) {
    console.error("Reject failed:", e);
  }
  requestOverlay.classList.add("hidden");
});

// ── Tauri event listeners ───────────────────────────────────────────────

listen("lan-devices-changed", (event) => {
  const devices = event.payload as Array<{
    name: string;
    peer_id: string;
    ip: string;
    port: number;
  }>;
  renderLanDevices(devices);
});

listen("connection-request", (event) => {
  const { device_name, pairing_code } = event.payload as {
    device_name: string;
    peer_id: string;
    pairing_code: string;
  };
  requestFrom.textContent = `${device_name} wants to connect`;
  requestCode.textContent = pairing_code;
  requestOverlay.classList.remove("hidden");
});

listen("connection-established", (event) => {
  const { peer_name, is_reconnect } = event.payload as {
    peer_name: string;
    peer_id: string;
    is_reconnect: boolean;
  };
  const how = is_reconnect ? "Reconnected to" : "Connected to";
  lastSync.textContent = `${how} ${peer_name} — ${formatTime()}`;
  setStatus("online");
  hideCodeOverlay();
  requestOverlay.classList.add("hidden");
});

listen("connection-failed", (event) => {
  lastSync.textContent = `Connection failed: ${event.payload}`;
  setStatus("offline");
  hideCodeOverlay();
  requestOverlay.classList.add("hidden");
});

listen("pairing-code-needed", () => {
  showCodeOverlay();
});

// ── Escape helper ───────────────────────────────────────────────────────

function esc(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// ── Startup ─────────────────────────────────────────────────────────────

loadPairingCode();
refreshStatus();
loadLanDevices();
setInterval(refreshStatus, 3000);
setInterval(loadLanDevices, 5000);

console.log("PlanarClip frontend ready");
