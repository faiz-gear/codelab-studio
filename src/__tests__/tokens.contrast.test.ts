import { describe, expect, it } from "vitest";
import tokensCss from "../styles/tokens.css?raw";

interface Rgb {
  r: number;
  g: number;
  b: number;
}

interface Rgba extends Rgb {
  a: number;
}

interface Oklab {
  L: number;
  a: number;
  b: number;
}

function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
}

function srgbToLinear(channel: number): number {
  return channel <= 0.04045
    ? channel / 12.92
    : Math.pow((channel + 0.055) / 1.055, 2.4);
}

function linearToSrgb(x: number): number {
  const clamped = clamp01(x);
  return clamped <= 0.0031308
    ? 12.92 * clamped
    : 1.055 * Math.pow(clamped, 1 / 2.4) - 0.055;
}

function oklabToRgb(lab: Oklab): Rgb {
  const l_ = lab.L + 0.3963377774 * lab.a + 0.2158037573 * lab.b;
  const m_ = lab.L - 0.1055613458 * lab.a - 0.0638541728 * lab.b;
  const s_ = lab.L - 0.0894841775 * lab.a - 1.291485548 * lab.b;

  const l = l_ * l_ * l_;
  const m = m_ * m_ * m_;
  const s = s_ * s_ * s_;

  const rLin = +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
  const gLin = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
  const bLin = -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s;

  return { r: linearToSrgb(rLin), g: linearToSrgb(gLin), b: linearToSrgb(bLin) };
}

function rgbToOklab(rgb: Rgb): Oklab {
  const r = srgbToLinear(rgb.r);
  const g = srgbToLinear(rgb.g);
  const b = srgbToLinear(rgb.b);

  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);

  return {
    L: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  };
}

function oklchToRgb(lPct: number, c: number, hDeg: number): Rgb {
  const L = lPct / 100;
  const hRad = (hDeg * Math.PI) / 180;
  return oklabToRgb({ L, a: c * Math.cos(hRad), b: c * Math.sin(hRad) });
}

function mixOklab(a: Rgb, b: Rgb, pctA: number): Rgb {
  const t = pctA / 100;
  const labA = rgbToOklab(a);
  const labB = rgbToOklab(b);
  return oklabToRgb({
    L: labA.L * t + labB.L * (1 - t),
    a: labA.a * t + labB.a * (1 - t),
    b: labA.b * t + labB.b * (1 - t),
  });
}

function compositeOver(fg: Rgba, bg: Rgb): Rgb {
  const a = clamp01(fg.a);
  return {
    r: fg.r * a + bg.r * (1 - a),
    g: fg.g * a + bg.g * (1 - a),
    b: fg.b * a + bg.b * (1 - a),
  };
}

function hexToRgb(hex: string): Rgb {
  const clean = hex.replace("#", "");
  return {
    r: parseInt(clean.slice(0, 2), 16) / 255,
    g: parseInt(clean.slice(2, 4), 16) / 255,
    b: parseInt(clean.slice(4, 6), 16) / 255,
  };
}

function parseRgba(value: string, vars: Record<string, string>): Rgba {
  const trimmed = value.trim();
  const varMatch = /^var\((--[a-z0-9-]+)\)$/i.exec(trimmed);
  if (varMatch) {
    const resolved = vars[varMatch[1]];
    if (!resolved) {
      throw new Error(`Unresolved CSS variable: ${varMatch[1]}`);
    }
    return parseRgba(resolved, vars);
  }
  if (trimmed.startsWith("#")) {
    return { ...hexToRgb(trimmed), a: 1 };
  }
  const oklchMatch =
    /^oklch\(\s*([\d.]+)%\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+))?\s*\)$/i.exec(
      trimmed
    );
  if (oklchMatch) {
    return {
      ...oklchToRgb(
        parseFloat(oklchMatch[1]),
        parseFloat(oklchMatch[2]),
        parseFloat(oklchMatch[3])
      ),
      a: oklchMatch[4] !== undefined ? parseFloat(oklchMatch[4]) : 1,
    };
  }
  const rgbMatch =
    /^rgb\(\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+))?\s*\)$/i.exec(
      trimmed
    );
  if (rgbMatch) {
    return {
      r: parseFloat(rgbMatch[1]) / 255,
      g: parseFloat(rgbMatch[2]) / 255,
      b: parseFloat(rgbMatch[3]) / 255,
      a: rgbMatch[4] !== undefined ? parseFloat(rgbMatch[4]) : 1,
    };
  }
  throw new Error(`Unsupported color token format: ${trimmed}`);
}

function parseColor(value: string, vars: Record<string, string>): Rgb {
  const rgba = parseRgba(value, vars);
  if (rgba.a < 1) {
    throw new Error(`Expected opaque color token, got alpha=${rgba.a}: ${value}`);
  }
  return { r: rgba.r, g: rgba.g, b: rgba.b };
}

function relativeLuminance(rgb: Rgb): number {
  const linearize = (channel: number) =>
    channel <= 0.03928
      ? channel / 12.92
      : Math.pow((channel + 0.055) / 1.055, 2.4);
  return (
    0.2126 * linearize(rgb.r) +
    0.7152 * linearize(rgb.g) +
    0.0722 * linearize(rgb.b)
  );
}

function contrastRatio(a: Rgb, b: Rgb): number {
  const l1 = relativeLuminance(a);
  const l2 = relativeLuminance(b);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

function extractVarsForBlock(
  css: string,
  selectorRegex: RegExp
): Record<string, string> {
  const match = selectorRegex.exec(css);
  if (!match) {
    throw new Error(`Missing CSS block for ${selectorRegex}`);
  }
  const vars: Record<string, string> = {};
  for (const prop of match[1].matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/gi)) {
    vars[prop[1]] = prop[2].trim();
  }
  return vars;
}

const rootBase = extractVarsForBlock(tokensCss, /:root\s*\{([\s\S]*?)\}/);
const lightVars = {
  ...rootBase,
  ...extractVarsForBlock(
    tokensCss,
    /:root,\s*:root\[data-theme="light"\]\s*\{([\s\S]*?)\}/
  ),
};
const darkVars = {
  ...rootBase,
  ...extractVarsForBlock(
    tokensCss,
    /:root\[data-theme="dark"\]\s*\{([\s\S]*?)\}/
  ),
};

describe("Arc UI v3.2 design tokens", () => {
  it("keeps all neutral scale tokens strictly zero-chroma", () => {
    for (let i = 0; i <= 11; i++) {
      expect(rootBase[`--neutral-${i}`]).toMatch(/^oklch\([\d.]+%\s+0\s+0\)$/);
    }
    for (const token of [
      "--background",
      "--surface",
      "--surface-raised",
      "--surface-muted",
      "--surface-subtle",
      "--border",
      "--border-subtle",
      "--border-strong",
    ]) {
      expect(darkVars[token]).toMatch(/^oklch\([\d.]+%\s+0\s+0\)$/);
    }
  });

  it("meets WCAG 2.1 AA (>= 4.5:1) for light and dark text, surfaces, semantic tones, danger buttons, badges, and syntax tokens", () => {
    const themes = [
      { name: "light", vars: lightVars },
      { name: "dark", vars: darkVars },
    ];
    const foregrounds = [
      "--foreground",
      "--text-secondary",
      "--text-muted",
      "--tone-teal",
      "--tone-sky",
      "--tone-orchid",
      "--tone-violet",
      "--tone-rose",
      "--tone-amber",
      "--tone-green",
    ];
    const surfaces = [
      "--background",
      "--surface",
      "--surface-raised",
      "--surface-muted",
    ];

    for (const theme of themes) {
      for (const fg of foregrounds) {
        for (const bg of surfaces) {
          const ratio = contrastRatio(
            parseColor(theme.vars[fg], theme.vars),
            parseColor(theme.vars[bg], theme.vars)
          );
          expect(
            ratio,
            `${theme.name} ${fg} on ${bg} was ${ratio.toFixed(2)}:1`
          ).toBeGreaterThanOrEqual(4.5);
        }
      }

      // Filled danger button (--danger-glyph on --danger) must meet AA in both Light & Dark
      const dangerBtnRatio = contrastRatio(
        parseColor(theme.vars["--danger-glyph"], theme.vars),
        parseColor(theme.vars["--danger"], theme.vars)
      );
      expect(
        dangerBtnRatio,
        `${theme.name} --danger-glyph on --danger was ${dangerBtnRatio.toFixed(2)}:1`
      ).toBeGreaterThanOrEqual(4.5);

      // Pastel badges, language tags, and soft AI buttons (color-mix in oklab)
      const tintPct = parseFloat(theme.vars["--badge-tint"]);
      const inkPct = parseFloat(theme.vars["--badge-ink"]);
      const surfaceRgb = parseColor(theme.vars["--surface"], theme.vars);
      const fgRgb = parseColor(theme.vars["--foreground"], theme.vars);
      for (const tone of [
        "--tone-teal",
        "--tone-sky",
        "--tone-orchid",
        "--tone-violet",
        "--tone-rose",
        "--tone-amber",
        "--tone-green",
        "--success",
        "--warning",
        "--danger",
        "--info",
        "--ai",
      ]) {
        const toneRgb = parseColor(theme.vars[tone], theme.vars);
        const badgeBg = mixOklab(toneRgb, surfaceRgb, tintPct);
        const badgeInk = mixOklab(toneRgb, fgRgb, inkPct);
        const ratio = contrastRatio(badgeInk, badgeBg);
        expect(
          ratio,
          `${theme.name} pastel badge ${tone} was ${ratio.toFixed(2)}:1`
        ).toBeGreaterThanOrEqual(4.5);
      }

      // Syntax tokens against editor --surface
      for (const syn of [
        "--syn-kw",
        "--syn-fn",
        "--syn-str",
        "--syn-num",
        "--syn-type",
        "--syn-com",
        "--syn-op",
      ]) {
        const ratio = contrastRatio(
          parseColor(theme.vars[syn], theme.vars),
          parseColor(theme.vars["--surface"], theme.vars)
        );
        expect(
          ratio,
          `${theme.name} ${syn} on --surface was ${ratio.toFixed(2)}:1`
        ).toBeGreaterThanOrEqual(4.5);
      }

      // --foreground and --text-secondary remain AA even on --surface-subtle
      for (const fg of ["--foreground", "--text-secondary"]) {
        const ratio = contrastRatio(
          parseColor(theme.vars[fg], theme.vars),
          parseColor(theme.vars["--surface-subtle"], theme.vars)
        );
        expect(
          ratio,
          `${theme.name} ${fg} on --surface-subtle was ${ratio.toFixed(2)}:1`
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it("meets WCAG 2.1 AA (>= 4.5:1) for hero secondary text across all hero gradient stops", () => {
    const lightStops = ["#ecd0d1", "#c9daf9", "#efe5c6", "#faf9f7"];
    for (const stop of lightStops) {
      const ratio = contrastRatio(
        parseColor(lightVars["--hero-text-secondary"], lightVars),
        hexToRgb(stop)
      );
      expect(
        ratio,
        `light --hero-text-secondary on hero stop ${stop} was ${ratio.toFixed(2)}:1`
      ).toBeGreaterThanOrEqual(4.5);
    }

    const darkStops = ["#543135", "#253654", "#443d2c", "#1c1d20"];
    for (const stop of darkStops) {
      const ratio = contrastRatio(
        parseColor(darkVars["--hero-text-secondary"], darkVars),
        hexToRgb(stop)
      );
      expect(
        ratio,
        `dark --hero-text-secondary on hero stop ${stop} was ${ratio.toFixed(2)}:1`
      ).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("meets WCAG 2.1 AA (>= 4.5:1) for all 8 accent presets across surfaces and soft-tint pills in light and dark themes", () => {
    const accents = [
      "neutral",
      "violet",
      "blue",
      "green",
      "amber",
      "orange",
      "coral",
      "rose",
    ];
    const surfaces = [
      "--background",
      "--surface",
      "--surface-raised",
      "--surface-muted",
    ];

    for (const accent of accents) {
      const lightAccent = {
        ...lightVars,
        ...extractVarsForBlock(
          tokensCss,
          new RegExp(`:root\\[data-accent="${accent}"\\]\\s*\\{([^}]*)\\}`)
        ),
      };
      const darkAccent = {
        ...darkVars,
        ...extractVarsForBlock(
          tokensCss,
          new RegExp(
            `:root\\[data-theme="dark"\\]\\[data-accent="${accent}"\\]\\s*\\{([^}]*)\\}`
          )
        ),
      };

      for (const [themeName, themeVars] of [
        ["light", lightAccent],
        ["dark", darkAccent],
      ] as const) {
        const btnRatio = contrastRatio(
          parseColor(themeVars["--control-glyph"], themeVars),
          parseColor(themeVars["--control-on"], themeVars)
        );
        expect(
          btnRatio,
          `${themeName} accent=${accent} control glyph/on was ${btnRatio.toFixed(2)}:1`
        ).toBeGreaterThanOrEqual(4.5);

        const accentRgb = parseColor(themeVars["--accent"], themeVars);
        const accentStrongRgb = parseColor(themeVars["--accent-strong"], themeVars);
        const accentSubtleRgba = parseRgba(themeVars["--accent-subtle"], themeVars);

        for (const surface of surfaces) {
          const bgRgb = parseColor(themeVars[surface], themeVars);
          const directRatio = contrastRatio(accentRgb, bgRgb);
          expect(
            directRatio,
            `${themeName} accent=${accent} --accent on ${surface} was ${directRatio.toFixed(2)}:1`
          ).toBeGreaterThanOrEqual(4.5);

          const subtleBg = compositeOver(accentSubtleRgba, bgRgb);
          const subtleRatio = contrastRatio(accentStrongRgb, subtleBg);
          expect(
            subtleRatio,
            `${themeName} accent=${accent} --accent-strong on --accent-subtle over ${surface} was ${subtleRatio.toFixed(2)}:1`
          ).toBeGreaterThanOrEqual(4.5);
        }
      }
    }
  });

  it("respects prefers-reduced-motion by collapsing all duration tokens to 0ms", () => {
    const reducedBlock = extractVarsForBlock(
      tokensCss,
      /@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{\s*:root\s*\{([\s\S]*?)\}/
    );
    for (const token of [
      "--duration-instant",
      "--duration-fast",
      "--duration-standard",
      "--duration-considered",
      "--duration-spring",
    ]) {
      expect(reducedBlock[token]).toBe("0ms");
    }
  });
});
