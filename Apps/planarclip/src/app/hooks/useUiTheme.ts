import { useCallback, useEffect } from "react";
import { getThemeById, normalizeColorScheme } from "../constants/theme";
import type { ColorScheme, CommandExecutor, ThemeColor, UiSettingsPayload } from "../types";
import {
  applyColorScheme,
  applyThemeColor,
  isDarkActive,
  savePreviewUiSettings,
} from "../utils/settings";
import { normalizeUserMessage } from "../utils/message";

type UseUiThemeOptions = {
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
  colorScheme: ColorScheme;
  theme: ThemeColor;
  setColorScheme: (scheme: ColorScheme) => void;
  setTheme: (theme: ThemeColor) => void;
  setIsDark: (isDark: boolean) => void;
  setSettingsMessage: (message: string) => void;
  setIsSavingSettings: (saving: boolean) => void;
};

/**
 * 管理主题同步、桌面端持久化与浏览器预览态的外观行为。
 * 输入：当前主题状态、状态 setter 与桌面命令执行器。
 * 输出：主题切换 handler，以及桌面端设置初始化/失败回退方法。
 */
export function useUiTheme({
  tauriAvailable,
  callCommand,
  colorScheme,
  theme,
  setColorScheme,
  setTheme,
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
    async (nextScheme: ColorScheme, nextTheme: ThemeColor) => {
      if (!tauriAvailable) {
        savePreviewUiSettings({
          color_scheme: nextScheme,
          theme_color: nextTheme.id,
        });
        setSettingsMessage("当前是浏览器预览模式，外观设置已暂存到浏览器本地。");
        return;
      }

      setIsSavingSettings(true);
      try {
        await callCommand<UiSettingsPayload>("save_ui_settings", {
          colorScheme: nextScheme,
          themeColor: nextTheme.id,
        });
        setSettingsMessage("外观设置已保存，下次打开桌面应用时会继续保留。");
      } catch (error) {
        setSettingsMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSavingSettings(false);
      }
    },
    [callCommand, setIsSavingSettings, setSettingsMessage, tauriAvailable],
  );

  const handleColorSchemeChange = useCallback(
    (nextScheme: ColorScheme) => {
      setColorScheme(nextScheme);
      void persistUiSettings(nextScheme, theme);
    },
    [persistUiSettings, setColorScheme, theme],
  );

  const handleThemeChange = useCallback(
    (nextTheme: ThemeColor) => {
      setTheme(nextTheme);
      void persistUiSettings(colorScheme, nextTheme);
    },
    [colorScheme, persistUiSettings, setTheme],
  );

  const applyDesktopUiSettings = useCallback(
    (uiSettings: UiSettingsPayload) => {
      setColorScheme(normalizeColorScheme(uiSettings.color_scheme));
      setTheme(getThemeById(uiSettings.theme_color));
      setSettingsMessage("桌面端设置已同步，可直接继续调整外观。");
    },
    [setColorScheme, setSettingsMessage, setTheme],
  );

  const applyUiSettingsFallback = useCallback(() => {
    setSettingsMessage("暂时还没有同步到桌面端设置，当前先使用默认外观。");
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
    applyDesktopUiSettings,
    applyUiSettingsFallback,
  };
}
