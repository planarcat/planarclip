/**
 * Free dev ports before `tauri dev` starts Vite + the desktop app.
 * Clears a leftover dev session (node/vite + planarclip-dev) on ports 1420/1421/19877.
 * Never touches planarclip.exe so the release build can run alongside dev.
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
  // Only free dev-session artifacts (vite/node + the dev binary itself).
  // planarclip.exe is the release build and must stay online alongside dev.
  return (
    name.includes("node") ||
    name === "planarclip-dev.exe" ||
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
    // Only stop a leftover dev binary; leave the release build (planarclip.exe) running.
    spawnSync("taskkill", ["/IM", "planarclip-dev.exe", "/F", "/T"], { stdio: "ignore" });
    return;
  }
  try {
    execSync(`pkill -x planarclip-dev`, { stdio: "ignore" });
  } catch {
    /* not running */
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
