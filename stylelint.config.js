/** @type {import("stylelint").Config} */
export default {
  extends: ["stylelint-config-standard"],
  rules: {
    "at-rule-no-unknown": [
      true,
      {
        ignoreAtRules: ["tailwind", "apply", "layer", "config"],
      },
    ],
    // Keep BEM/Arc double-dash modifiers (.btn--primary, .seg--pill) and CSS custom prop conventions intact
    "selector-class-pattern": null,
    "custom-property-empty-line-before": null,
    "comment-empty-line-before": null,
    "rule-empty-line-before": null,
    "declaration-empty-line-before": null,
    "alpha-value-notation": null,
    "color-function-notation": null,
    "color-hex-length": null,
    "hue-degree-notation": null,
    "lightness-notation": null,
    "value-keyword-case": null,
    "font-family-name-quotes": null,
    "media-feature-range-notation": null,
    "shorthand-property-no-redundant-values": null,
    // Strictly enforce cascade & duplicate-override bug catchers
    "no-descending-specificity": true,
    "no-duplicate-selectors": true,
    "declaration-block-no-duplicate-properties": true,
    "declaration-block-no-shorthand-property-overrides": true,
  },
};
