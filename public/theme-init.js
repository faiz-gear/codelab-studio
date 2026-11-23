(function () {
  var themes = ["light", "dark"];
  var accents = ["neutral", "violet", "blue", "green", "amber", "orange", "coral", "rose"];
  var theme = "light";
  var accent = "neutral";
  try {
    var savedTheme = window.localStorage.getItem("codelab.theme");
    if (savedTheme && themes.indexOf(savedTheme) !== -1) {
      theme = savedTheme;
    }
    var savedAccent = window.localStorage.getItem("codelab.accent");
    if (savedAccent && accents.indexOf(savedAccent) !== -1) {
      accent = savedAccent;
    }
  } catch (_err) {
    // Ignore storage access errors in restricted contexts.
  }
  document.documentElement.dataset.theme = theme;
  document.documentElement.dataset.accent = accent;
})();
