import { invoke } from "@tauri-apps/api/core";

const statusDot = document.getElementById("statusDot")!;
const statusText = document.getElementById("statusText")!;
const lastSync = document.getElementById("lastSync")!;
const pairInput = document.getElementById("pairInput") as HTMLInputElement;
const pairBtn = document.getElementById("pairBtn") as HTMLButtonElement;
const myCodeEl = document.getElementById("myCode")!;
const myCodeSection = document.getElementById("myCodeSection")!;

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
  const now = new Date();
  return now.toLocaleTimeString();
}

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

// On startup
loadPairingCode();
refreshStatus();
setInterval(refreshStatus, 3000);

console.log("PlanarClip frontend ready");
