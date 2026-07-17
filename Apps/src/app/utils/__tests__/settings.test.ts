import { beforeEach, describe, expect, it } from "vitest";

import {
  loadPreviewUiSettings,
  normalizeDeviceName,
  savePreviewUiSettings,
} from "../settings";
import {
  DEFAULT_DEVICE_NAME,
  DEFAULT_UI_SETTINGS,
  PREVIEW_UI_SETTINGS_KEY,
} from "../../constants/theme";

// jsdom 已提供 window.localStorage，用例前清一次，保证隔离。
beforeEach(() => {
  window.localStorage.clear();
});

describe("normalizeDeviceName", () => {
  it("空字符串回退到预览默认名", () => {
    const name = normalizeDeviceName("");
    expect(name.length).toBeGreaterThan(0);
  });

  it("默认占位名 / `my device` 被视为空", () => {
    // 覆盖 storage 未初始化时的占位名逻辑
    expect(normalizeDeviceName(DEFAULT_DEVICE_NAME)).not.toBe(DEFAULT_DEVICE_NAME);
    expect(normalizeDeviceName("My Device")).not.toBe("My Device");
  });

  it("正常名字原样返回（trim）", () => {
    expect(normalizeDeviceName("  planarcat-win11  ")).toBe("planarcat-win11");
  });
});

describe("loadPreviewUiSettings", () => {
  it("无 storage 时返回默认设置，device_name 用预览兜底", () => {
    const s = loadPreviewUiSettings();
    expect(s.color_scheme).toBe(DEFAULT_UI_SETTINGS.color_scheme);
    expect(s.theme_color).toBe(DEFAULT_UI_SETTINGS.theme_color);
    expect(s.device_name.length).toBeGreaterThan(0);
  });

  it("合法 JSON 会被规范化后返回", () => {
    window.localStorage.setItem(
      PREVIEW_UI_SETTINGS_KEY,
      JSON.stringify({
        color_scheme: "light",
        theme_color: "violet",
        device_name: "hostA",
      }),
    );
    const s = loadPreviewUiSettings();
    expect(s.color_scheme).toBe("light");
    expect(s.theme_color).toBe("violet");
    expect(s.device_name).toBe("hostA");
  });

  it("非法 color_scheme / theme_color 回退到默认值", () => {
    window.localStorage.setItem(
      PREVIEW_UI_SETTINGS_KEY,
      JSON.stringify({
        color_scheme: "nope",
        theme_color: "unknown",
        device_name: "hostA",
      }),
    );
    const s = loadPreviewUiSettings();
    expect(s.color_scheme).toBe(DEFAULT_UI_SETTINGS.color_scheme);
    expect(s.theme_color).toBe(DEFAULT_UI_SETTINGS.theme_color);
  });

  it("损坏的 JSON 回退到默认，不抛异常", () => {
    window.localStorage.setItem(PREVIEW_UI_SETTINGS_KEY, "{ this is not json");
    const s = loadPreviewUiSettings();
    expect(s.color_scheme).toBe(DEFAULT_UI_SETTINGS.color_scheme);
    expect(s.theme_color).toBe(DEFAULT_UI_SETTINGS.theme_color);
    expect(s.device_name.length).toBeGreaterThan(0);
  });
});

describe("savePreviewUiSettings", () => {
  it("写入后可被 load 读回，device_name 会被规范化", () => {
    savePreviewUiSettings({
      color_scheme: "dark",
      theme_color: "emerald",
      device_name: "  my-mac  ",
    });
    const raw = window.localStorage.getItem(PREVIEW_UI_SETTINGS_KEY);
    expect(raw).not.toBeNull();

    const s = loadPreviewUiSettings();
    expect(s.color_scheme).toBe("dark");
    expect(s.theme_color).toBe("emerald");
    expect(s.device_name).toBe("my-mac");
  });
});