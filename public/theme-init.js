(function hydrateAppearanceCache() {
  var CACHE_KEY = "codelab.appearance.v1";
  var VALID_THEMES = { light: true, dark: true };
  var VALID_ACCENTS = {
    neutral: true,
    violet: true,
    blue: true,
    green: true,
    amber: true,
    orange: true,
    coral: true,
    rose: true,
  };
  var VALID_LOCALES = { "zh-CN": true, "en-US": true };

  try {
    var raw = window.localStorage.getItem(CACHE_KEY);
    if (!raw) return;
    var parsed = JSON.parse(raw);
    var root = document.documentElement;
    if (parsed && VALID_THEMES[parsed.theme]) {
      root.dataset.theme = parsed.theme;
    }
    if (parsed && VALID_ACCENTS[parsed.accent]) {
      root.dataset.accent = parsed.accent;
    }
    if (
      parsed &&
      typeof parsed.codeFontSize === "number" &&
      isFinite(parsed.codeFontSize) &&
      parsed.codeFontSize >= 11 &&
      parsed.codeFontSize <= 20
    ) {
      root.style.setProperty("--code-size", parsed.codeFontSize + "px");
      root.style.setProperty(
        "--code-lh",
        Math.round(parsed.codeFontSize + 8.5) + "px"
      );
    }
    if (parsed && VALID_LOCALES[parsed.locale]) {
      root.lang = parsed.locale;
    }
  } catch {
    // Ignore storage access or parse errors and fall back to default tokens.
  }
})();
