import { invoke, isTauri } from "@tauri-apps/api/core";
import { useCallback, useMemo, useState } from "react";
import { DevicesPanel } from "./components/layout/DevicesPanel";
import { Sidebar } from "./components/layout/Sidebar";
import { ConnectionAttemptCard } from "./components/overlays/ConnectionAttemptCard";
import { IncomingConnectionPrompt } from "./components/overlays/IncomingConnectionPrompt";
import { PairingModal } from "./components/overlays/PairingModal";
import { SwitchConnectionPrompt } from "./components/overlays/SwitchConnectionPrompt";
import { StatusNotice } from "./components/overlays/StatusNotice";
import { ClipboardPage } from "./components/pages/ClipboardPage";
import { DevicesPage } from "./components/pages/DevicesPage";
import { SettingsPage } from "./components/pages/SettingsPage";
import { getThemeById, normalizeColorScheme } from "./constants/theme";
import { useClipboardSettings } from "./hooks/useClipboardSettings";
import { useConnectionBridge } from "./hooks/useConnectionBridge";
import { useConnectionSettings } from "./hooks/useConnectionSettings";
import { useSyncSettings } from "./hooks/useSyncSettings";
import { usePairingFlow } from "./hooks/usePairingFlow";
import { useStartupSettings } from "./hooks/useStartupSettings";
import { useUiTheme } from "./hooks/useUiTheme";
import type {
  AppConnectionStatus,
  ClipEntry,
  ColorScheme,
  CommandExecutor,
  ConnectedPeer,
  ConnectionEstablishedPayload,
  ConnectionRequestPayload,
  Device,
  LanDevicePayload,
  NavId,
  PairingCodeRotatedPayload,
  PairingStage,
  ThemeColor,
  TrustedPeerPayload,
  ViewMode,
} from "./types";
import { buildDevices } from "./utils/device";
import { MSG_PAIRING_CODE_REFRESHED, normalizeUserMessage } from "./utils/message";
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
  const [deviceName, setDeviceName] = useState(previewUiSettings.device_name);
  const [isDark, setIsDark] = useState(true);
  const [, setSettingsMessage] = useState(
    TAURI_AVAILABLE ? "正在同步桌面端设置…" : "当前是浏览器预览模式，外观设置会暂存到浏览器本地。",
  );
  const [isSavingSettings, setIsSavingSettings] = useState(false);
  const [status, setStatus] = useState<AppConnectionStatus>(TAURI_AVAILABLE ? "connecting" : "offline");
  const [lastMessage, setLastMessage] = useState(
    TAURI_AVAILABLE ? "正在初始化连接桥接…" : "当前是浏览器预览模式，连接能力需在桌面应用中体验。",
  );
  const [pairingCode, setPairingCode] = useState("------");
  const [lanDevices, setLanDevices] = useState<LanDevicePayload[]>([]);
  const [trustedPeers, setTrustedPeers] = useState<TrustedPeerPayload[]>([]);
  const [connectedPeer, setConnectedPeer] = useState<ConnectedPeer | null>(null);
  const [showPairing, setShowPairing] = useState(false);
  const [pairingInput, setPairingInput] = useState("");
  const [pairingStage, setPairingStage] = useState<PairingStage>("idle");
  const [pairingTarget, setPairingTarget] = useState<Device | null>(null);
  const [pairingHelperText, setPairingHelperText] = useState("通过配对码或设备列表建立连接。");
  const [pairingError, setPairingError] = useState<string | null>(null);
  const [pairingRotationHint, setPairingRotationHint] = useState<string | null>(null);
  const [incomingRequest, setIncomingRequest] = useState<ConnectionRequestPayload | null>(null);
  const [noticeMessage, setNoticeMessage] = useState<string | null>(null);
  const [isRefreshingDevices, setIsRefreshingDevices] = useState(false);

  const devices = useMemo(() => buildDevices(lanDevices, connectedPeer, trustedPeers), [lanDevices, connectedPeer, trustedPeers]);
  const connectedCount = useMemo(() => devices.filter((device) => device.status === "connected").length, [devices]);
  const onlineDeviceCount = useMemo(() => devices.filter((device) => device.status !== "offline").length, [devices]);
  const discoveredDevices = useMemo(
    () => devices.filter((device) => device.source === "discovery" && device.status !== "connected"),
    [devices],
  );
  const identityLabel = deviceName;
  const showConnectionAttemptCard = pairingStage === "requesting_device" && pairingTarget != null;
  const showPairingModal = showPairing && pairingStage !== "requesting_device";

  const {
    handleColorSchemeChange,
    handleThemeChange,
    handleDeviceNameChange,
    handleDeviceNameSave,
    applyDesktopUiSettings,
    applyUiSettingsFallback,
  } = useUiTheme({
    tauriAvailable: TAURI_AVAILABLE,
    callCommand,
    colorScheme,
    theme,
    deviceName,
    setColorScheme,
    setTheme,
    setDeviceName,
    setIsDark,
    setSettingsMessage,
    setIsSavingSettings,
  });

  const {
    launchAtStartup,
    silentStart,
    isSavingStartupSettings,
    startupSettingsLoaded,
    handleLaunchAtStartupChange,
    handleSilentStartChange,
  } = useStartupSettings({
    tauriAvailable: TAURI_AVAILABLE,
    callCommand,
    setLastMessage,
  });

  const {
    autoConnectTrusted,
    isSavingConnectionSettings,
    connectionSettingsLoaded,
    handleAutoConnectTrustedChange,
  } = useConnectionSettings({
    tauriAvailable: TAURI_AVAILABLE,
    callCommand,
    setLastMessage,
  });

  const {
    syncImages,
    syncFiles,
    isSavingSyncSettings,
    syncSettingsLoaded,
    handleSyncImagesChange,
    handleSyncFilesChange,
  } = useSyncSettings({
    tauriAvailable: TAURI_AVAILABLE,
    callCommand,
    setLastMessage,
  });

  const {
    historyLimit,
    isSavingClipboardSettings,
    clipboardSettingsLoaded,
    isClearingClipboardHistory,
    handleHistoryLimitChange,
    handleClearHistory,
  } = useClipboardSettings({
    tauriAvailable: TAURI_AVAILABLE,
    callCommand,
    setLastMessage,
    setClips,
  });

  const refreshLanDevicesQuiet = useCallback(async () => {
    if (!TAURI_AVAILABLE) {
      return;
    }

    try {
      const refreshedDevices = await callCommand<LanDevicePayload[]>("refresh_lan_devices");
      setLanDevices(refreshedDevices);
    } catch {
    }
  }, [setLanDevices]);

  const {
    openPairingModal,
    closePairingModal,
    handleConnectLan,
    switchPairingTarget,
    handleSubmitPairingCode,
    handleRotatePairingCode,
    handleAcceptIncoming,
    handleRejectIncoming,
    handleIncomingResponseTimeout,
    handleDisconnect,
    handleConnectionRequest,
    handleConnectionEstablished,
    handleConnectionFailed,
    handleConnectionEnded,
    handleOutboundConnectionStarted,
    handleOutboundConnectionPending,
    handlePairingCodeNeeded,
    switchConnectionTarget,
    confirmSwitchConnection,
    cancelSwitchConnection,
    pairingStageRef,
    connectionLocked,
  } = usePairingFlow({
    callCommand,
    status,
    connectedCount,
    pairingInput,
    pairingStage,
    pairingTarget,
    incomingRequest,
    setStatus,
    setLastMessage,
    setConnectedPeer,
    setShowPairing,
    setPairingInput,
    setPairingStage,
    setPairingTarget,
    setPairingHelperText,
    setPairingError,
    setPairingRotationHint,
    setPairingCode,
    setIncomingRequest,
    refreshLanDevices: refreshLanDevicesQuiet,
    showNotice: setNoticeMessage,
  });

  const handleRefreshDevices = useCallback(async () => {
    if (!TAURI_AVAILABLE) {
      setLastMessage("当前是浏览器预览模式，设备刷新需在桌面应用中体验。");
      return;
    }

    setIsRefreshingDevices(true);

    try {
      const refreshedDevices = await callCommand<LanDevicePayload[]>("refresh_lan_devices");
      setLanDevices(refreshedDevices);
      setLastMessage(
        refreshedDevices.length > 0
          ? `已刷新，当前发现 ${refreshedDevices.length} 台设备。`
          : "已刷新，暂无发现更多设备。",
      );
    } catch (error) {
      setLastMessage(normalizeUserMessage(error, "刷新失败，请稍后重试。"));
    } finally {
      setIsRefreshingDevices(false);
    }
  }, [setLanDevices, setLastMessage]);

  const refreshTrustedPeers = useCallback(async () => {
    if (!TAURI_AVAILABLE) {
      return;
    }

    try {
      const peers = await callCommand<TrustedPeerPayload[]>("get_trusted_peers");
      setTrustedPeers(peers);
    } catch (error) {
      setLastMessage(normalizeUserMessage(error, "读取熟悉设备列表失败，请稍后重试。"));
    }
  }, [setLastMessage]);

  const handleRemoveTrustedPeer = useCallback(
    async (device: Device) => {
      if (!device.peerId) {
        setLastMessage("这个设备缺少标识，暂时无法移除。");
        return;
      }

      const wasConnected = device.status === "connected" || connectedPeer?.peerId === device.peerId;

      try {
        const removed = await callCommand<boolean>("remove_trusted_peer", { peerId: device.peerId });
        await refreshTrustedPeers();

        if (!removed) {
          setLastMessage(`没有找到 ${device.name} 的记录。`);
          return;
        }

        if (wasConnected) {
          await handleDisconnect();
          return;
        }
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, `移除 ${device.name} 失败，请稍后重试。`, device.name));
      }
    },
    [connectedPeer, handleDisconnect, refreshTrustedPeers, setLastMessage],
  );

  const handleSetPeerAutoAccept = useCallback(
    async (device: Device, autoAccept: boolean) => {
      if (!device.peerId) {
        setLastMessage("这个设备缺少标识，暂时无法更新信任设置。");
        return;
      }

      try {
        const updated = await callCommand<boolean>("set_peer_auto_accept", {
          peerId: device.peerId,
          autoAccept,
        });
        await refreshTrustedPeers();
        if (updated) {
          setLastMessage(
            autoAccept
              ? `已信任 ${device.name}，对方发起连接时将自动接受。`
              : `已改为不信任 ${device.name}，对方下次连接需要你确认。`,
          );
        }
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, `更新 ${device.name} 的信任设置失败，请稍后重试。`, device.name));
      }
    },
    [refreshTrustedPeers, setLastMessage],
  );

  const handleTrustedConnectionEstablished = useCallback(
    (payload: ConnectionEstablishedPayload) => {
      handleConnectionEstablished(payload);
      void refreshTrustedPeers();
    },
    [handleConnectionEstablished, refreshTrustedPeers],
  );

  const handlePairingCodeRotated = useCallback(
    (payload: PairingCodeRotatedPayload) => {
      setPairingRotationHint(MSG_PAIRING_CODE_REFRESHED);
      setPairingCode(payload.code);
    },
    [setPairingCode],
  );

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
    setTrustedPeers,
    setClips,
    setConnectedPeer,
    applyDesktopUiSettings,
    applyUiSettingsFallback,
    toUserMessage: normalizeUserMessage,
    onConnectionRequest: handleConnectionRequest,
    onConnectionEstablished: handleTrustedConnectionEstablished,
    onConnectionFailed: handleConnectionFailed,
    onConnectionEnded: handleConnectionEnded,
    onOutboundConnectionStarted: handleOutboundConnectionStarted,
    onOutboundConnectionPending: handleOutboundConnectionPending,
    onPairingCodeNeeded: handlePairingCodeNeeded,
    onPairingCodeRotated: handlePairingCodeRotated,
  });

  return (
    <div className="flex h-screen w-full overflow-hidden bg-background text-foreground">
      <Sidebar
        activeNav={activeNav}
        identityLabel={identityLabel}
        connectedDeviceCount={connectedCount}
        onlineDeviceCount={onlineDeviceCount}
        colorScheme={colorScheme}
        setColorScheme={handleColorSchemeChange}
        theme={theme}
        isDark={isDark}
        isSavingDeviceName={isSavingSettings}
        onThemeChange={handleThemeChange}
        onNavigate={setActiveNav}
        onDeviceNameChange={handleDeviceNameChange}
        onDeviceNameSave={handleDeviceNameSave}
      />

      <main className="flex h-full min-w-0 flex-1 overflow-hidden">
        {activeNav === "clipboard" && (
          <>
            <ClipboardPage
              clips={clips}
              viewMode={viewMode}
              setViewMode={setViewMode}
              status={status}
              statusMessage={lastMessage}
              isClearingHistory={isClearingClipboardHistory}
              onClearHistory={() => {
                void handleClearHistory();
              }}
            />
            <DevicesPanel
              devices={devices}
              connectionStatus={status}
              connectionLocked={connectionLocked}
              connectedCount={connectedCount}
              onConnectDevice={(device) => {
                void handleConnectLan(device);
              }}
              onDisconnect={() => {
                void handleDisconnect();
              }}
            />
          </>
        )}
        {activeNav === "devices" && (
          <DevicesPage
            devices={devices}
            connectionStatus={status}
            onShowPairing={openPairingModal}
            onRefreshDevices={() => {
              void handleRefreshDevices();
            }}
            onConnectDevice={(device) => {
              void handleConnectLan(device);
            }}
            onDisconnect={() => {
              void handleDisconnect();
            }}
            onRemoveTrustedPeer={(device) => {
              void handleRemoveTrustedPeer(device);
            }}
            onSetPeerAutoAccept={(device, autoAccept) => {
              void handleSetPeerAutoAccept(device, autoAccept);
            }}
            isRefreshingDevices={isRefreshingDevices}
          />
        )}
        {activeNav === "settings" && (
          <SettingsPage
            colorScheme={colorScheme}
            deviceName={deviceName}
            isDark={isDark}
            theme={theme}
            isSaving={isSavingSettings}
            launchAtStartup={launchAtStartup}
            silentStart={silentStart}
            isSavingStartupSettings={isSavingStartupSettings}
            startupSettingsLoaded={startupSettingsLoaded}
            onSchemeChange={handleColorSchemeChange}
            onThemeChange={handleThemeChange}
            onDeviceNameChange={handleDeviceNameChange}
            onDeviceNameSave={handleDeviceNameSave}
            onLaunchAtStartupChange={handleLaunchAtStartupChange}
            onSilentStartChange={handleSilentStartChange}
            autoConnectTrusted={autoConnectTrusted}
            isSavingConnectionSettings={isSavingConnectionSettings}
            connectionSettingsLoaded={connectionSettingsLoaded}
            onAutoConnectTrustedChange={handleAutoConnectTrustedChange}
            syncImages={syncImages}
            syncFiles={syncFiles}
            isSavingSyncSettings={isSavingSyncSettings}
            syncSettingsLoaded={syncSettingsLoaded}
            onSyncImagesChange={handleSyncImagesChange}
            onSyncFilesChange={handleSyncFilesChange}
            clipboardHistoryLimit={historyLimit}
            isSavingClipboardSettings={isSavingClipboardSettings}
            clipboardSettingsLoaded={clipboardSettingsLoaded}
            onClipboardHistoryLimitChange={(limit) => {
              void handleHistoryLimitChange(limit);
            }}
          />
        )}
      </main>

      {incomingRequest &&
        (pairingStage === "incoming_request" || pairingStage === "incoming_accepting") && (
        <IncomingConnectionPrompt
          request={incomingRequest}
          accepting={pairingStage === "incoming_accepting"}
          onAccept={() => {
            void handleAcceptIncoming();
          }}
          onReject={() => {
            void handleRejectIncoming();
          }}
          onTimeout={() => {
            void handleIncomingResponseTimeout();
          }}
        />
      )}

      {showPairingModal && (
        <PairingModal
          initialTarget={pairingTarget}
          allDiscoverable={discoveredDevices}
          pairingCode={pairingCode.padEnd(6, "•").slice(0, 6)}
          input={pairingInput}
          stage={pairingStage}
          helperText={pairingHelperText}
          errorMessage={pairingError}
          rotationHint={pairingRotationHint}
          connectionLocked={connectionLocked}
          onClose={() => {
            void closePairingModal();
          }}
          onInputChange={setPairingInput}
          onSelectDevice={(device) => {
            void switchPairingTarget(device);
          }}
          onSubmitPairingCode={() => {
            void handleSubmitPairingCode();
          }}
          onRotatePairingCode={() => {
            void handleRotatePairingCode();
          }}
        />
      )}

      {switchConnectionTarget && connectedPeer && (
        <SwitchConnectionPrompt
          currentDeviceName={connectedPeer.name}
          targetDevice={switchConnectionTarget}
          onConfirm={() => {
            void confirmSwitchConnection();
          }}
          onCancel={cancelSwitchConnection}
        />
      )}

      {showConnectionAttemptCard && pairingTarget && (
        <ConnectionAttemptCard deviceName={pairingTarget.name} />
      )}

      {noticeMessage && (
        <StatusNotice message={noticeMessage} onDismiss={() => setNoticeMessage(null)} />
      )}
    </div>
  );
}
