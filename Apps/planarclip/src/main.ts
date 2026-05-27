import { invoke } from "@tauri-apps/api/core";

const statusDot = document.getElementById("statusDot")!;
const statusText = document.getElementById("statusText")!;
const lastSync = document.getElementById("lastSync")!;
const pairInput = document.getElementById("pairInput") as HTMLInputElement;
const pairBtn = document.getElementById("pairBtn")!;
const myCodeEl = document.getElementById("myCode")!;
const myCodeSection = document.getElementById("myCodeSection")!;

let connected = false;

function setStatus(state: "offline" | "connecting" | "online") {
  statusDot.className = `status-dot ${state}`;
  const labels: Record<string, string> = {
    offline: "Offline",
    connecting: "Connecting...",
    online: "Connected",
  };
  statusText.textContent = labels[state];
}

async function refreshStatus() {
  try {
    const status = await invoke<string>("get_status");
    if (status === "connected") {
      connected = true;
      setStatus("online");
    }
  } catch (e) {
    console.error("Status check failed:", e);
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
  // TODO: initiate pairing via Tauri command
});

// On startup
refreshStatus();
setInterval(refreshStatus, 3000);

console.log("PlanarClip frontend ready");
