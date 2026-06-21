import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const statusDot = document.getElementById("statusDot")!;
const statusText = document.getElementById("statusText")!;
const lastSync = document.getElementById("lastSync")!;
const pairInput = document.getElementById("pairInput") as HTMLInputElement;
const pairBtn = document.getElementById("pairBtn") as HTMLButtonElement;
const myCodeEl = document.getElementById("myCode")!;
const myCodeSection = document.getElementById("myCodeSection")!;
const lanList = document.getElementById("lanList")!;

const codeOverlay = document.getElementById("codeOverlay")!;
const codeInput = document.getElementById("codeInput") as HTMLInputElement;
const codeSubmit = document.getElementById("codeSubmit") as HTMLButtonElement;
const codeCancel = document.getElementById("codeCancel") as HTMLButtonElement;
const codeError = document.getElementById("codeError")!;

const requestOverlay = document.getElementById("requestOverlay")!;
const requestFrom = document.getElementById("requestFrom")!;
const requestCode = document.getElementById("requestCode")!;
const requestReject = document.getElementById("requestReject") as HTMLButtonElement;

type UiStatus = "offline" | "connecting" | "online";
type LanDevice = { name: string; peer_id: string; ip: string; port: number };
type ConnectionEventPayload = {
  kind?: string;
  message?: string;
  peer_name?: string;
};

let currentStatus: UiStatus = "offline";
let pendingLanTargetName = "这台设备";
let lanBusy = false;

function syncLanActionState() {
  lanList.querySelectorAll<HTMLButtonElement>(".lan-connect").forEach((button) => {
    button.disabled = lanBusy || currentStatus === "online";
  });
}

function setLanBusy(busy: boolean) {
  lanBusy = busy;
  syncLanActionState();
}

function setStatus(state: UiStatus) {
  currentStatus = state;
  statusDot.className = `status-dot ${state}`;
  const labels: Record<UiStatus, string> = {
    offline: "未连接",
    connecting: "连接中...",
    online: "已连接",
  };
  statusText.textContent = labels[state];
  syncLanActionState();
}

function setLastMessage(message: string) {
  lastSync.textContent = message;
}

function formatTime() {
  return new Date().toLocaleTimeString();
}

function rawMessage(error: unknown) {
  if (error && typeof error === "object") {
    const payload = error as ConnectionEventPayload;
    if (typeof payload.message === "string" && payload.message.trim()) {
      return payload.message.trim();
    }
  }

  return String(error ?? "").trim();
}

function normalizeUserMessage(error: unknown, fallback: string) {
  const raw = rawMessage(error);

  if (!raw) {
    return fallback;
  }

  if (raw.includes("配对码必须为 6 位数字")) {
    return "请输入 6 位数字配对码。";
  }

  if (raw.includes("当前没有待处理的连接")) {
    return "这次连接已经结束，请重新选择设备后再试。";
  }

  if (raw.includes("密钥对尚未初始化")) {
    return "设备还在准备连接信息，请稍后再试。";
  }

  if (raw.includes("对方已拒绝连接") || raw.includes("对方已拒绝这次连接")) {
    return "对方没有继续这次连接，请重新发起连接。";
  }

  if (raw.includes("配对码已过期") || raw.includes("这次连接已超时")) {
    return "这次配对已超时，请重新发起连接并输入新的配对码。";
  }

  if (raw.includes("配对码无效") || raw.includes("配对码不正确")) {
    return "配对码不正确，或这次连接已经失效，请重新核对后再试。";
  }

  if (raw.includes("已取消") || raw.includes("用户已取消")) {
    return "这次连接已经取消，你可以重新选择设备。";
  }

  if (raw.includes("已断开连接")) {
    return raw;
  }

  if (
    raw.includes("I/O 错误") ||
    raw.includes("I/O error") ||
    raw.includes("os error 10061") ||
    raw.includes("无法连接") ||
    raw.includes("actively refused") ||
    raw.includes("暂时无法连接对方设备")
  ) {
    return `暂时连不上 ${pendingLanTargetName}，请确认对方应用已打开，而且你们在同一局域网内。`;
  }

  if (raw.includes("协议错误") || raw.includes("帧错误") || raw.includes("连接过程中出了点问题")) {
    return "连接过程中出了点问题，请重新发起连接。";
  }

  if (raw.startsWith("连接失败：") || raw.startsWith("配对失败：")) {
    return normalizeUserMessage(raw.replace(/^[^：]+：/, "").trim(), fallback);
  }

  return fallback;
}

async function refreshStatus() {
  try {
    const status = await invoke<string>("get_status");
    if (status === "connected") {
      setStatus("online");
      return;
    }

    if (!lanBusy && currentStatus !== "connecting") {
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
    setLastMessage("请输入有效的 6 位配对码。");
    return;
  }

  setLastMessage(`正在根据配对码 ${code} 建立连接...`);
  setStatus("connecting");
  pairBtn.disabled = true;

  try {
    await invoke<string>("pair", { code });
    setLastMessage(`已完成配对，连接已建立 — ${formatTime()}`);
    setStatus("online");
  } catch (e) {
    setLastMessage(normalizeUserMessage(e, "这次配对没有成功，请稍后再试。"));
    setStatus("offline");
  } finally {
    pairBtn.disabled = false;
  }
});

async function loadLanDevices() {
  try {
    const devices = await invoke<Array<LanDevice>>("get_lan_devices");
    renderLanDevices(devices);
  } catch (e) {
    console.error("Failed to load LAN devices:", e);
  }
}

function renderLanDevices(devices: Array<LanDevice>) {
  if (devices.length === 0) {
    lanList.innerHTML = '<div class="empty-hint">暂时还没有发现附近设备</div>';
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
      <button class="lan-connect" data-name="${esc(d.name)}" data-ip="${esc(d.ip)}" data-port="${d.port}">连接设备</button>
    </div>`,
    )
    .join("");

  lanList.querySelectorAll(".lan-connect").forEach((btn) => {
    btn.addEventListener("click", async (e) => {
      const target = e.currentTarget as HTMLElement;
      const name = target.dataset.name || "这台设备";
      const ip = target.dataset.ip!;
      const port = Number.parseInt(target.dataset.port!, 10);
      await startLanConnect(name, ip, port);
    });
  });

  syncLanActionState();
}

async function startLanConnect(name: string, ip: string, port: number) {
  if (lanBusy || currentStatus === "connecting") {
    setLastMessage("正在处理上一次连接，请稍候...");
    return;
  }

  if (currentStatus === "online") {
    setLastMessage("当前已经建立连接，如需切换设备，请先断开当前连接。")
    return;
  }

  pendingLanTargetName = name;
  setLanBusy(true);
  setLastMessage(`正在请求连接 ${name}，请稍候...`);
  setStatus("connecting");

  try {
    const result = await invoke<string>("connect_lan", { ip, port });
    if (result === "awaiting_code") {
      showCodeOverlay(`请查看 ${name} 屏幕上的 6 位配对码，并在这里输入。`);
      return;
    }

    setLanBusy(false);
  } catch (e) {
    setLanBusy(false);
    setLastMessage(normalizeUserMessage(e, `暂时无法连接 ${name}，请稍后重试。`));
    setStatus("offline");
  }
}

function showCodeOverlay(message?: string) {
  codeOverlay.classList.remove("hidden");
  codeInput.value = "";
  codeError.textContent = "";
  if (message) {
    setLastMessage(message);
  }
  setStatus("connecting");
  codeInput.focus();
}

function hideCodeOverlay() {
  codeOverlay.classList.add("hidden");
  codeError.textContent = "";
}

codeSubmit.addEventListener("click", async () => {
  const code = codeInput.value.trim();
  if (code.length !== 6 || !/^\d+$/.test(code)) {
    codeError.textContent = "请输入 6 位数字配对码。";
    return;
  }

  codeSubmit.disabled = true;
  codeError.textContent = "";
  setLanBusy(true);
  setLastMessage("正在核对配对码，请稍候...");
  setStatus("connecting");

  try {
    await invoke<string>("submit_pairing_code", { code });
    hideCodeOverlay();
  } catch (e) {
    const message = normalizeUserMessage(e, "这次连接没有成功，请重新发起连接。");
    codeError.textContent = message;
    setLastMessage(message);
    setLanBusy(false);
    setStatus("offline");
  } finally {
    codeSubmit.disabled = false;
  }
});

codeCancel.addEventListener("click", async () => {
  try {
    await invoke("disconnect");
  } catch (e) {
    console.error("Cancel failed:", e);
  }

  hideCodeOverlay();
  setLanBusy(false);
  setLastMessage("已取消本次连接，你可以重新选择附近设备。");
  setStatus("offline");
});

requestReject.addEventListener("click", async () => {
  try {
    await invoke("reject_connection");
    setLastMessage("已拒绝这次连接请求。");
  } catch (e) {
    console.error("Reject failed:", e);
    setLastMessage("拒绝连接时出了点问题，请稍后再试。");
  }
  requestOverlay.classList.add("hidden");
  setLanBusy(false);
  setStatus("offline");
});

listen("lan-devices-changed", (event) => {
  const devices = event.payload as Array<LanDevice>;
  renderLanDevices(devices);
});

listen("connection-request", (event) => {
  const { device_name, pairing_code } = event.payload as {
    device_name: string;
    peer_id: string;
    pairing_code: string;
  };
  pendingLanTargetName = device_name;
  requestFrom.textContent = `${device_name} 想要连接这台设备`;
  requestCode.textContent = pairing_code;
  requestOverlay.classList.remove("hidden");
  setLanBusy(true);
  setLastMessage(`请让 ${device_name} 在对方设备上输入屏幕里的 6 位配对码。`);
  setStatus("connecting");
});

listen("connection-established", (event) => {
  const { peer_name, is_reconnect } = event.payload as {
    peer_name: string;
    peer_id: string;
    is_reconnect: boolean;
  };
  pendingLanTargetName = peer_name;
  setLanBusy(false);
  setLastMessage(
    is_reconnect
      ? `已重新连上 ${peer_name}，现在可以继续同步剪贴板了 — ${formatTime()}`
      : `已与 ${peer_name} 建立连接，现在可以开始同步剪贴板了 — ${formatTime()}`,
  );
  setStatus("online");
  hideCodeOverlay();
  requestOverlay.classList.add("hidden");
});

listen("connection-failed", (event) => {
  setLanBusy(false);
  setLastMessage(normalizeUserMessage(event.payload, "这次连接没有成功，请重新发起连接。"));
  setStatus("offline");
  hideCodeOverlay();
  requestOverlay.classList.add("hidden");
});

listen("connection-ended", (event) => {
  setLanBusy(false);
  setLastMessage(normalizeUserMessage(event.payload, "连接已断开，请重新连接。"));
  setStatus("offline");
  hideCodeOverlay();
  requestOverlay.classList.add("hidden");
});

listen("pairing-code-needed", () => {
  showCodeOverlay(`请查看 ${pendingLanTargetName} 屏幕上的 6 位配对码，并在这里输入。`);
});

function esc(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

loadPairingCode();
refreshStatus();
loadLanDevices();
setInterval(refreshStatus, 3000);
setInterval(loadLanDevices, 5000);

console.log("PlanarClip frontend ready");
