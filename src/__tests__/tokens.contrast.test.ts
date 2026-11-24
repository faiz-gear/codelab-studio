import { describe, expect, it } from "vitest";
import tokensCss from "../styles/tokens.css?raw";

interface Rgb {
  r: number;
  g: number;
  b: number;
}

function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
}

function oklchToRgb(lPct: number, c: number, hDeg: number): Rgb {
  const L = lPct / 100;
  const hRad = (hDeg * Math.PI) / 180;
  const a = c * Math.cos(hRad);
  const b = c * Math.sin(hRad);

  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.291485548 * b;

  const l = l_ * l_ * l_;
  const m = m_ * m_ * m_;
  const s = s_ * s_ * s_;

  const rLin = +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
  const gLin = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
  const bLin = -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s;

  const encode = (x: number) => {
    const clamped = clamp01(x);
    return clamped <= 0.0031308
      ? 12.92 * clamped
      : 1.055 * Math.pow(clamped, 1 / 2.4) - 0.055;
  };

  return { r: encode(rLin), g: encode(gLin), b: encode(bLin) };
}

function hexToRgb(hex: string): Rgb {
  const clean = hex.replace("#", "");
  return {
    r: parseInt(clean.slice(0, 2), 16) / 255,
    g: parseInt(clean.slice(2, 4), 16) / 255,
    b: parseInt(clean.slice(4, 6), 16) / 255,
  };
}

function parseColor(value: string, vars: Record<string, string>): Rgb {
  const trimmed = value.trim();
  const varMatch = /^var\((--[a-z0-9-]+)\)$/i.exec(trimmed);
  if (varMatch) {
    const resolved = vars[varMatch[1]];
    if (!resolved) {
      throw new Error(`Unresolved CSS variable: ${varMatch[1]}`);
    }
    return parseColor(resolved, vars);
  }
  if (trimmed.startsWith("#")) {
    return hexToRgb(trimmed);
  }
  const oklchMatch =
    /^oklch\(\s*([\d.]+)%\s+([\d.]+)\s+([\d.]+)\s*\)$/i.exec(trimmed);
  if (oklchMatch) {
    return oklchToRgb(
      parseFloat(oklchMatch[1]),
      parseFloat(oklchMatch[2]),
      parseFloat(oklchMatch[3])
    );
  }
  throw new Error(`Unsupported color token format: ${trimmed}`);
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

function extractVarsForBlock(css: string, selectorRegex: RegExp): Record<string, string> {
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
  ...extractVarsForBlock(tokensCss, /:root\[data-theme="dark"\]\s*\{([\s\S]*?)\}/),
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

  it("meets WCAG 2.1 AA (>= 4.5:1) for light and dark text and semantic tones", () => {
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
      "--tone-violet",
      "--tone-rose",
      "--tone-amber",
      "--tone-green",
    ];
    const surfaces = ["--background", "--surface", "--surface-muted"];

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
    }
  });

  it("meets WCAG 2.1 AA (>= 4.5:1) for all 8 accent presets in light and dark themes", () => {
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

      const lightBtnRatio = contrastRatio(
        parseColor(lightAccent["--control-glyph"], lightAccent),
        parseColor(lightAccent["--control-on"], lightAccent)
      );
      expect(
        lightBtnRatio,
        `light accent=${accent} control glyph/on was ${lightBtnRatio.toFixed(2)}:1`
      ).toBeGreaterThanOrEqual(4.5);

      const lightAccentTextRatio = contrastRatio(
        parseColor(lightAccent["--accent"], lightAccent),
        parseColor(lightAccent["--surface"], lightAccent)
      );
      expect(
        lightAccentTextRatio,
        `light accent=${accent} on surface was ${lightAccentTextRatio.toFixed(2)}:1`
      ).toBeGreaterThanOrEqual(4.5);

      const darkBtnRatio = contrastRatio(
        parseColor(darkAccent["--control-glyph"], darkAccent),
        parseColor(darkAccent["--control-on"], darkAccent)
      );
      expect(
        darkBtnRatio,
        `dark accent=${accent} control glyph/on was ${darkBtnRatio.toFixed(2)}:1`
      ).toBeGreaterThanOrEqual(4.5);

      const darkAccentTextRatio = contrastRatio(
        parseColor(darkAccent["--accent"], darkAccent),
        parseColor(darkAccent["--surface"], darkAccent)
      );
      expect(
        darkAccentTextRatio,
        `dark accent=${accent} on surface was ${darkAccentTextRatio.toFixed(2)}:1`
      ).toBeGreaterThanOrEqual(4.5);
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
