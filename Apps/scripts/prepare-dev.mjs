/**
 * Free dev ports before `tauri dev` starts Vite + the desktop app.
 * Avoids "Port 1420 is already in use" when a previous dev session left node/planarclip running.
 */
import { execSync, spawnSync } from "node:child_process";
import { platform } from "node:os";

const PORTS = [1420, 1421, 19877];

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function pidsListeningOnPort(port) {
  const sys = platform();
  const pids = new Set();

  if (sys === "win32") {
    try {
      const out = execSync("netstat -ano -p tcp", { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
      for (const line of out.split(/\r?\n/)) {
        if (!line.includes("LISTENING")) continue;
        if (!line.includes(`:${port}`)) continue;
        const parts = line.trim().split(/\s+/);
        const pid = Number.parseInt(parts[parts.length - 1], 10);
        if (Number.isFinite(pid) && pid > 0) pids.add(pid);
      }
    } catch {
      /* nothing listening or netstat unavailable */
    }
    return pids;
  }

  try {
    const out = execSync(`lsof -nP -iTCP:${port} -sTCP:LISTEN -t`, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    for (const line of out.split(/\r?\n/)) {
      const pid = Number.parseInt(line.trim(), 10);
      if (Number.isFinite(pid) && pid > 0) pids.add(pid);
    }
  } catch {
    /* port free */
  }
  return pids;
}

function processName(pid) {
  const sys = platform();
  try {
    if (sys === "win32") {
      const out = execSync(`tasklist /FI "PID eq ${pid}" /FO CSV /NH`, {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      });
      const match = out.match(/^"([^"]+)"/);
      return match?.[1]?.toLowerCase() ?? "";
    }
    const out = execSync(`ps -p ${pid} -o comm=`, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    return out.trim().toLowerCase();
  } catch {
    return "";
  }
}

function shouldKill(name) {
  if (!name) return false;
  return (
    name.includes("node") ||
    name === "planarclip.exe" ||
    name === "planarclip-dev.exe" ||
    name === "planarclip" ||
    name === "planarclip-dev"
  );
}

function killPid(pid) {
  const sys = platform();
  if (sys === "win32") {
    spawnSync("taskkill", ["/PID", String(pid), "/F", "/T"], { stdio: "ignore" });
    return;
  }
  try {
    process.kill(pid, "SIGTERM");
  } catch {
    /* already gone */
  }
}

function killDevDesktopProcesses() {
  const sys = platform();
  if (sys === "win32") {
    for (const image of ["planarclip.exe", "planarclip-dev.exe"]) {
      spawnSync("taskkill", ["/IM", image, "/F", "/T"], { stdio: "ignore" });
    }
    return;
  }
  for (const name of ["planarclip-dev", "planarclip"]) {
    try {
      execSync(`pkill -x ${name}`, { stdio: "ignore" });
    } catch {
      /* not running */
    }
  }
}

killDevDesktopProcesses();
sleep(400);

for (const port of PORTS) {
  for (const pid of pidsListeningOnPort(port)) {
    const name = processName(pid);
    if (!shouldKill(name)) continue;
    console.log(`[prepare-dev] stopping ${name} (pid ${pid}) on port ${port}`);
    killPid(pid);
  }
}

sleep(400);
