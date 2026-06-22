import { useCallback, useEffect } from "react";
import { getThemeById, normalizeColorScheme } from "../constants/theme";
import type { ColorScheme, CommandExecutor, ThemeColor, UiSettingsPayload } from "../types";
import {
  applyColorScheme,
  applyThemeColor,
  isDarkActive,
  normalizeDeviceName,
  savePreviewUiSettings,
} from "../utils/settings";
import { normalizeUserMessage } from "../utils/message";

type PersistMessage = {
  desktop: string;
  preview: string;
};

type UseUiThemeOptions = {
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
  colorScheme: ColorScheme;
  theme: ThemeColor;
  deviceName: string;
  setColorScheme: (scheme: ColorScheme) => void;
  setTheme: (theme: ThemeColor) => void;
  setDeviceName: (deviceName: string) => void;
  setIsDark: (isDark: boolean) => void;
  setSettingsMessage: (message: string) => void;
  setIsSavingSettings: (saving: boolean) => void;
};

/**
 * 管理主题同步、桌面端持久化与浏览器预览态的设置行为。
 * 输入：当前主题/设备名称状态、状态 setter 与桌面命令执行器。
 * 输出：外观与设备名称的变更 handler，以及桌面端设置初始化/失败回退方法。
 */
export function useUiTheme({
  tauriAvailable,
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
}: UseUiThemeOptions) {
  const syncTheme = useCallback(
    (nextTheme: ThemeColor, nextScheme: ColorScheme) => {
      applyColorScheme(nextScheme);
      applyThemeColor(nextTheme);
      setIsDark(isDarkActive());
    },
    [setIsDark],
  );

  const persistUiSettings = useCallback(
    async (
      nextScheme: ColorScheme,
      nextTheme: ThemeColor,
      nextDeviceName: string,
      message: PersistMessage,
    ) => {
      const normalizedName = normalizeDeviceName(nextDeviceName);

      if (!tauriAvailable) {
        savePreviewUiSettings({
          color_scheme: nextScheme,
          theme_color: nextTheme.id,
          device_name: normalizedName,
        });
        setDeviceName(normalizedName);
        setSettingsMessage(message.preview);
        return;
      }

      setIsSavingSettings(true);
      try {
        const savedSettings = await callCommand<UiSettingsPayload>("save_ui_settings", {
          colorScheme: nextScheme,
          themeColor: nextTheme.id,
          deviceName: normalizedName,
        });
        setDeviceName(normalizeDeviceName(savedSettings.device_name));
        setSettingsMessage(message.desktop);
      } catch (error) {
        setSettingsMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSavingSettings(false);
      }
    },
    [callCommand, setDeviceName, setIsSavingSettings, setSettingsMessage, tauriAvailable],
  );

  const handleColorSchemeChange = useCallback(
    (nextScheme: ColorScheme) => {
      setColorScheme(nextScheme);
      void persistUiSettings(nextScheme, theme, deviceName, {
        desktop: "外观设置已保存，下次打开桌面应用时会继续保留。",
        preview: "当前是浏览器预览模式，外观设置已暂存到浏览器本地。",
      });
    },
    [deviceName, persistUiSettings, setColorScheme, theme],
  );

  const handleThemeChange = useCallback(
    (nextTheme: ThemeColor) => {
      setTheme(nextTheme);
      void persistUiSettings(colorScheme, nextTheme, deviceName, {
        desktop: "外观设置已保存，下次打开桌面应用时会继续保留。",
        preview: "当前是浏览器预览模式，外观设置已暂存到浏览器本地。",
      });
    },
    [colorScheme, deviceName, persistUiSettings, setTheme],
  );

  const handleDeviceNameChange = useCallback(
    (nextDeviceName: string) => {
      setDeviceName(nextDeviceName.slice(0, 24));
    },
    [setDeviceName],
  );

  const handleDeviceNameSave = useCallback(() => {
    const nextDeviceName = normalizeDeviceName(deviceName);
    setDeviceName(nextDeviceName);
    void persistUiSettings(colorScheme, theme, nextDeviceName, {
      desktop: "设备名称已保存，新的名称会用于后续连接。",
      preview: "当前是浏览器预览模式，设备名称已暂存到浏览器本地。",
    });
  }, [colorScheme, deviceName, persistUiSettings, setDeviceName, theme]);

  const applyDesktopUiSettings = useCallback(
    (uiSettings: UiSettingsPayload) => {
      setColorScheme(normalizeColorScheme(uiSettings.color_scheme));
      setTheme(getThemeById(uiSettings.theme_color));
      setDeviceName(normalizeDeviceName(uiSettings.device_name));
      setSettingsMessage("桌面端设置已同步，可直接继续调整外观与设备名称。");
    },
    [setColorScheme, setDeviceName, setSettingsMessage, setTheme],
  );

  const applyUiSettingsFallback = useCallback(() => {
    setSettingsMessage("暂时还没有同步到桌面端设置，当前先使用默认外观与设备名称。");
  }, [setSettingsMessage]);

  useEffect(() => {
    syncTheme(theme, colorScheme);
  }, [colorScheme, syncTheme, theme]);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = () => {
      if (colorScheme === "system") {
        syncTheme(theme, "system");
      }
    };

    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, [colorScheme, syncTheme, theme]);

  return {
    handleColorSchemeChange,
    handleThemeChange,
    handleDeviceNameChange,
    handleDeviceNameSave,
    applyDesktopUiSettings,
    applyUiSettingsFallback,
  };
}
