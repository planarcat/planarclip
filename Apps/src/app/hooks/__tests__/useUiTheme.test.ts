import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useUiTheme } from "../useUiTheme";
import type { ColorScheme, ThemeColor, UiSettingsPayload } from "../../types";
import { getThemeById } from "../../constants/theme";

vi.mock("../../utils/settings", () => ({
  applyColorScheme: vi.fn(),
  applyThemeColor: vi.fn(),
  isDarkActive: () => false,
  normalizeDeviceName: (name?: string) => (name?.trim().slice(0, 24) || "未知设备"),
  savePreviewUiSettings: vi.fn(),
  applyAppearanceFromUiSettings: vi.fn(),
}));
vi.mock("../../utils/appearanceBootstrap", () => ({
  mirrorUiSettingsForBootstrap: vi.fn(),
}));
vi.mock("../../utils/message", () => ({
  normalizeUserMessage: (_: unknown, fallback: string) => fallback,
}));

const DEFAULT_THEME: ThemeColor = getThemeById("cyan");
const ALT_THEME: ThemeColor = getThemeById("violet");

function makeMocks() {
  const callCommand = vi.fn();
  const setColorScheme = vi.fn();
  const setTheme = vi.fn();
  const setDeviceName = vi.fn();
  const setIsDark = vi.fn();
  const setSettingsMessage = vi.fn();
  const setIsSavingSettings = vi.fn();
  return {
    callCommand, setColorScheme, setTheme, setDeviceName,
    setIsDark, setSettingsMessage, setIsSavingSettings,
  };
}

function renderUiTheme(overrides: Partial<ReturnType<typeof makeMocks>> & { tauriAvailable?: boolean; suspendThemeSync?: boolean; colorScheme?: ColorScheme; deviceName?: string } = {}) {
  const defaults = makeMocks();
  const mocks = { ...defaults, ...overrides };
  return {
    mocks,
    ...renderHook(() => useUiTheme({
      tauriAvailable: overrides.tauriAvailable ?? false,
      callCommand: mocks.callCommand,
      colorScheme: overrides.colorScheme ?? ("system" as ColorScheme),
      theme: DEFAULT_THEME,
      deviceName: overrides.deviceName ?? "MyPC",
      setColorScheme: mocks.setColorScheme,
      setTheme: mocks.setTheme,
      setDeviceName: mocks.setDeviceName,
      setIsDark: mocks.setIsDark,
      setSettingsMessage: mocks.setSettingsMessage,
      setIsSavingSettings: mocks.setIsSavingSettings,
      suspendThemeSync: overrides.suspendThemeSync ?? false,
    })),
  };
}

describe("useUiTheme", () => {
  describe("handleColorSchemeChange", () => {
    it("浏览器预览模式：更新本地状态并保存到 localStorage", async () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: false });
      const { savePreviewUiSettings } = await import("../../utils/settings");

      await act(async () => {
        result.current.handleColorSchemeChange("dark");
      });

      expect(mocks.setColorScheme).toHaveBeenCalledWith("dark");
      expect(savePreviewUiSettings).toHaveBeenCalled();
      expect(mocks.setSettingsMessage).toHaveBeenCalledWith("当前是浏览器预览模式，外观设置已暂存到浏览器本地。");
      expect(mocks.callCommand).not.toHaveBeenCalled();
    });

    it("桌面端：调用 save 并更新", async () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: true });
      mocks.callCommand.mockResolvedValue({
        color_scheme: "dark",
        theme_color: "cyan",
        device_name: "MyPC",
      });

      await act(async () => {
        result.current.handleColorSchemeChange("dark");
      });

      expect(mocks.callCommand).toHaveBeenCalledWith("save_ui_settings", {
        colorScheme: "dark",
        themeColor: "cyan",
        deviceName: "MyPC",
      });
      expect(mocks.setSettingsMessage).toHaveBeenCalledWith("外观设置已保存，下次打开桌面应用时会继续保留。");
    });

    it("桌面端保存失败时显示错误提示", async () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: true });
      mocks.callCommand.mockRejectedValue(new Error("save failed"));

      await act(async () => {
        result.current.handleColorSchemeChange("dark");
      });

      expect(mocks.setSettingsMessage).toHaveBeenCalledWith("这次没有保存成功，请稍后再试。");
    });
  });

  describe("handleThemeChange", () => {
    it("浏览器预览模式：更新本地状态", async () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: false });

      await act(async () => {
        result.current.handleThemeChange(ALT_THEME);
      });

      expect(mocks.setTheme).toHaveBeenCalledWith(ALT_THEME);
    });

    it("桌面端：调用 save 并更新", async () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: true });
      mocks.callCommand.mockResolvedValue({
        color_scheme: "system",
        theme_color: "violet",
        device_name: "MyPC",
      });

      await act(async () => {
        result.current.handleThemeChange(ALT_THEME);
      });

      expect(mocks.callCommand).toHaveBeenCalledWith("save_ui_settings", {
        colorScheme: "system",
        themeColor: "violet",
        deviceName: "MyPC",
      });
    });
  });

  describe("handleDeviceNameChange", () => {
    it("截断到 24 字符", () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: false });

      act(() => {
        result.current.handleDeviceNameChange("a".repeat(50));
      });

      expect(mocks.setDeviceName).toHaveBeenCalledWith("a".repeat(24));
    });

    it("短名称保持原样", () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: false });

      act(() => {
        result.current.handleDeviceNameChange("ShortName");
      });

      expect(mocks.setDeviceName).toHaveBeenCalledWith("ShortName");
    });
  });

  describe("handleDeviceNameSave", () => {
    it("浏览器预览模式：保存到 localStorage", async () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: false, deviceName: "MyPC" });
      const { savePreviewUiSettings } = await import("../../utils/settings");

      await act(async () => {
        result.current.handleDeviceNameSave();
      });

      expect(mocks.setDeviceName).toHaveBeenCalledWith("MyPC");
      expect(savePreviewUiSettings).toHaveBeenCalled();
      expect(mocks.setSettingsMessage).toHaveBeenCalledWith("当前是浏览器预览模式，设备名称已暂存到浏览器本地。");
    });

    it("桌面端：调用 save 并更新", async () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: true, deviceName: "MyPC" });
      mocks.callCommand.mockResolvedValue({
        color_scheme: "system",
        theme_color: "cyan",
        device_name: "MyPC",
      });

      await act(async () => {
        result.current.handleDeviceNameSave();
      });

      expect(mocks.callCommand).toHaveBeenCalledWith("save_ui_settings", {
        colorScheme: "system",
        themeColor: "cyan",
        deviceName: "MyPC",
      });
      expect(mocks.setSettingsMessage).toHaveBeenCalledWith("设备名称已保存，新的名称会用于后续连接。");
    });

    it("传入新名称时使用该名称", async () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: false, deviceName: "OldName" });

      await act(async () => {
        result.current.handleDeviceNameSave("NewDevice");
      });

      expect(mocks.setDeviceName).toHaveBeenCalledWith("NewDevice");
    });
  });

  describe("applyDesktopUiSettings", () => {
    it("应用所有设置并更新状态", () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: true });

      const settings: UiSettingsPayload = {
        color_scheme: "dark",
        theme_color: "violet",
        device_name: "DesktopPC",
      };

      act(() => {
        result.current.applyDesktopUiSettings(settings);
      });

      expect(mocks.setColorScheme).toHaveBeenCalledWith("dark");
      expect(mocks.setDeviceName).toHaveBeenCalledWith("DesktopPC");
      expect(mocks.setIsDark).toHaveBeenCalled();
      expect(mocks.setSettingsMessage).toHaveBeenCalledWith("桌面端设置已同步，可直接继续调整外观与设备名称。");
    });
  });

  describe("applyUiSettingsFallback", () => {
    it("显示回退提示", () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: true });

      act(() => {
        result.current.applyUiSettingsFallback();
      });

      expect(mocks.setSettingsMessage).toHaveBeenCalledWith("暂时还没有同步到桌面端设置，当前先使用默认外观与设备名称。");
    });
  });

  describe("persistAppearanceSettings", () => {
    it("浏览器预览模式：保存到 localStorage", async () => {
      const { result, mocks } = renderUiTheme({ tauriAvailable: false, colorScheme: "light", deviceName: "MyPC" });

      await act(async () => {
        result.current.persistAppearanceSettings("dark", ALT_THEME);
      });

      expect(mocks.setSettingsMessage).toHaveBeenCalledWith("当前是浏览器预览模式，外观设置已暂存到浏览器本地。");
    });
  });

  describe("suspendThemeSync", () => {
    it("为 true 时不触发 syncTheme 副作用", () => {
      const { mocks } = renderUiTheme({ tauriAvailable: true, suspendThemeSync: true });

      expect(mocks.setIsDark).not.toHaveBeenCalled();
    });
  });

  describe("matchMedia 系统颜色变化监听", () => {
    it("colorScheme=system 时注册监听器", () => {
      const addEventListener = vi.fn();
      const removeEventListener = vi.fn();
      const matchMediaMock = vi.fn(() => ({
        matches: false,
        addEventListener,
        removeEventListener,
        media: "(prefers-color-scheme: dark)",
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      }));
      vi.stubGlobal("matchMedia", matchMediaMock);

      renderUiTheme({ tauriAvailable: true, colorScheme: "system" });

      expect(matchMediaMock).toHaveBeenCalledWith("(prefers-color-scheme: dark)");
      expect(addEventListener).toHaveBeenCalledWith("change", expect.any(Function));
      vi.unstubAllGlobals();
    });
  });
});
