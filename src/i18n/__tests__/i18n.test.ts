import { describe, expect, it } from "vitest";
import { enUS } from "../en-US";
import { isLocale, resolveInitialLocale, t } from "../index";
import { zhCN } from "../zh-CN";

describe("i18n", () => {
  it("resolves zh* navigator languages to zh-CN and everything else to en-US", () => {
    expect(resolveInitialLocale("zh")).toBe("zh-CN");
    expect(resolveInitialLocale("zh-CN")).toBe("zh-CN");
    expect(resolveInitialLocale("zh-TW")).toBe("zh-CN");
    expect(resolveInitialLocale("zh-Hans-SG")).toBe("zh-CN");
    expect(resolveInitialLocale("  ZH-hk  ")).toBe("zh-CN");

    expect(resolveInitialLocale("en")).toBe("en-US");
    expect(resolveInitialLocale("en-US")).toBe("en-US");
    expect(resolveInitialLocale("ja-JP")).toBe("en-US");
    expect(resolveInitialLocale("")).toBe("en-US");
    expect(resolveInitialLocale(null)).toBe("en-US");
    expect(resolveInitialLocale(undefined)).toBe("en-US");
  });

  it("validates locales with isLocale type guard", () => {
    expect(isLocale("zh-CN")).toBe(true);
    expect(isLocale("en-US")).toBe(true);
    expect(isLocale("zh-TW")).toBe(false);
    expect(isLocale(null)).toBe(false);
    expect(isLocale(42)).toBe(false);
  });

  it("returns localized strings and interpolates parameters", () => {
    expect(t("en-US", "app.name")).toBe("CodeLab Studio");
    expect(t("zh-CN", "shell.theme.light")).toBe("浅色");
    expect(t("en-US", "shell.theme.dark")).toBe("Dark");

    expect(t("en-US", "error.unknown", { detail: "timeout" })).toBe(
      "An unknown error occurred: timeout",
    );
    expect(t("zh-CN", "error.invalidInput", { field: "port" })).toBe("输入参数无效：port");
    expect(t("en-US", "error.invalidInput", { field: 1420 })).toBe("Invalid input: 1420");
    expect(t("en-US", "error.unknown", {})).toBe("An unknown error occurred: {detail}");
  });

  it("keeps zh-CN and en-US dictionaries aligned with non-empty values", () => {
    const zhKeys = Object.keys(zhCN).sort();
    const enKeys = Object.keys(enUS).sort();
    expect(zhKeys).toEqual(enKeys);

    for (const value of Object.values(zhCN)) {
      expect(value.trim().length).toBeGreaterThan(0);
    }
    for (const value of Object.values(enUS)) {
      expect(value.trim().length).toBeGreaterThan(0);
    }
  });
});
