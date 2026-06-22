import { invoke, isTauri } from "@tauri-apps/api/core";
import { useMemo, useState } from "react";
import { DevicesPanel } from "./components/layout/DevicesPanel";
import { Sidebar } from "./components/layout/Sidebar";
import { PairingModal } from "./components/overlays/PairingModal";
import { ClipboardPage } from "./components/pages/ClipboardPage";
import { DevicesPage } from "./components/pages/DevicesPage";
import { SettingsPage } from "./components/pages/SettingsPage";
import { getThemeById, normalizeColorScheme } from "./constants/theme";
import { useConnectionBridge } from "./hooks/useConnectionBridge";
import { usePairingFlow } from "./hooks/usePairingFlow";
import { useUiTheme } from "./hooks/useUiTheme";
import type {
  AppConnectionStatus,
  ClipEntry,
  ColorScheme,
  CommandExecutor,
  ConnectedPeer,
  ConnectionRequestPayload,
  LanDevicePayload,
  NavId,
  PairingStage,
  ThemeColor,
  ViewMode,
} from "./types";
import { buildDevices } from "./utils/device";
import { normalizeUserMessage } from "./utils/message";
import { loadPreviewUiSettings } from "./utils/settings";

const TAURI_AVAILABLE = isTauri();
const EMPTY_CLIPS: ClipEntry[] = [];

const callCommand: CommandExecutor = async function <T>(command: string, args?: Record<string, unknown>) {
  if (!TAURI_AVAILABLE) {
    throw new Error("当前是浏览器预览模式，请在桌面应用中体验连接能力。");
  }

  return invoke<T>(command, args);
};

export default function App() {
  const previewUiSettings = useMemo(() => loadPreviewUiSettings(), []);
  const [activeNav, setActiveNav] = useState<NavId>("clipboard");
  const [clips, setClips] = useState<ClipEntry[]>(EMPTY_CLIPS);
  const [viewMode, setViewMode] = useState<ViewMode>("list");
  const [theme, setTheme] = useState<ThemeColor>(() => getThemeById(previewUiSettings.theme_color));
  const [colorScheme, setColorScheme] = useState<ColorScheme>(() => normalizeColorScheme(previewUiSettings.color_scheme));
  const [isDark, setIsDark] = useState(true);
  const [settingsMessage, setSettingsMessage] = useState(
    TAURI_AVAILABLE ? "正在同步桌面端设置…" : "当前是浏览器预览模式，外观设置会暂存到浏览器本地。",
  );
  const [isSavingSettings, setIsSavingSettings] = useState(false);
  const [status, setStatus] = useState<AppConnectionStatus>(TAURI_AVAILABLE ? "connecting" : "offline");
  const [lastMessage, setLastMessage] = useState(
    TAURI_AVAILABLE ? "正在初始化连接桥接…" : "当前是浏览器预览模式，连接能力需在桌面应用中体验。",
  );
  const [pairingCode, setPairingCode] = useState("------");
  const [lanDevices, setLanDevices] = useState<LanDevicePayload[]>([]);
  const [connectedPeer, setConnectedPeer] = useState<ConnectedPeer | null>(null);
  const [showPairing, setShowPairing] = useState(false);
  const [pairingInput, setPairingInput] = useState("");
  const [pairingStage, setPairingStage] = useState<PairingStage>("idle");
  const [pairingTargetName, setPairingTargetName] = useState<string | null>(null);
  const [pairingHelperText, setPairingHelperText] = useState("通过配对码或局域网设备建立连接。");
  const [pairingError, setPairingError] = useState<string | null>(null);
  const [incomingRequest, setIncomingRequest] = useState<ConnectionRequestPayload | null>(null);

  const devices = useMemo(() => buildDevices(lanDevices, connectedPeer), [lanDevices, connectedPeer]);
  const discoveredDevices = useMemo(() => devices.filter((device) => device.source === "discovery"), [devices]);
  const identityLabel = pairingCode === "------" ? "连接信息准备中" : `配对码 ${pairingCode}`;

  const { handleColorSchemeChange, handleThemeChange, applyDesktopUiSettings, applyUiSettingsFallback } = useUiTheme({
    tauriAvailable: TAURI_AVAILABLE,
    callCommand,
    colorScheme,
    theme,
    setColorScheme,
    setTheme,
    setIsDark,
    setSettingsMessage,
    setIsSavingSettings,
  });

  const {
    openPairingModal,
    closePairingModal,
    handleManualPair,
    handleConnectLan,
    handleSubmitPairingCode,
    handleRejectIncoming,
    handleDisconnect,
    handleConnectionRequest,
    handleConnectionEstablished,
    handleConnectionFailed,
    handleConnectionEnded,
    pairingStageRef,
  } = usePairingFlow({
    callCommand,
    status,
    pairingInput,
    pairingStage,
    pairingTargetName,
    incomingRequest,
    setStatus,
    setLastMessage,
    setConnectedPeer,
    setShowPairing,
    setPairingInput,
    setPairingStage,
    setPairingTargetName,
    setPairingHelperText,
    setPairingError,
    setIncomingRequest,
  });

  useConnectionBridge({
    tauriAvailable: TAURI_AVAILABLE,
    callCommand,
    status,
    connectedPeer,
    pairingStageRef,
    setStatus,
    setLastMessage,
    setPairingCode,
    setLanDevices,
    setClips,
    setConnectedPeer,
    applyDesktopUiSettings,
    applyUiSettingsFallback,
    toUserMessage: normalizeUserMessage,
    onConnectionRequest: handleConnectionRequest,
    onConnectionEstablished: handleConnectionEstablished,
    onConnectionFailed: handleConnectionFailed,
    onConnectionEnded: handleConnectionEnded,
  });

  return (
    <div className="flex h-screen w-full overflow-hidden bg-background text-foreground">
      <Sidebar
        activeNav={activeNav}
        devices={devices}
        status={status}
        identityLabel={identityLabel}
        colorScheme={colorScheme}
        setColorScheme={handleColorSchemeChange}
        theme={theme}
        isDark={isDark}
        onThemeChange={handleThemeChange}
        onNavigate={setActiveNav}
        tauriAvailable={TAURI_AVAILABLE}
      />

      <main className="flex h-full min-w-0 flex-1 overflow-hidden">
        {activeNav === "clipboard" && (
          <>
            <ClipboardPage
              clips={clips}
              devices={devices}
              viewMode={viewMode}
              setViewMode={setViewMode}
              status={status}
              statusMessage={lastMessage}
            />
            <DevicesPanel devices={devices} pairingCode={pairingCode} status={status} />
          </>
        )}
        {activeNav === "devices" && (
          <DevicesPage
            devices={devices}
            connectionStatus={status}
            onShowPairing={openPairingModal}
            onConnectDevice={(device) => {
              void handleConnectLan(device);
            }}
            onDisconnect={() => {
              void handleDisconnect();
            }}
          />
        )}
        {activeNav === "settings" && (
          <SettingsPage
            colorScheme={colorScheme}
            isDark={isDark}
            theme={theme}
            settingsMessage={settingsMessage}
            isSaving={isSavingSettings}
            onSchemeChange={handleColorSchemeChange}
            onThemeChange={handleThemeChange}
          />
        )}
      </main>

      {showPairing && (
        <PairingModal
          pairingCode={pairingCode.padEnd(6, "•").slice(0, 6)}
          input={pairingInput}
          stage={pairingStage}
          discoveredDevices={discoveredDevices}
          helperText={pairingHelperText}
          errorMessage={pairingError}
          incomingRequest={incomingRequest}
          onClose={() => {
            void closePairingModal();
          }}
          onInputChange={setPairingInput}
          onManualPair={() => {
            void handleManualPair();
          }}
          onConnectLan={(device) => {
            void handleConnectLan(device);
          }}
          onSubmitPairingCode={() => {
            void handleSubmitPairingCode();
          }}
          onRejectIncoming={() => {
            void handleRejectIncoming();
          }}
        />
      )}
    </div>
  );
}
