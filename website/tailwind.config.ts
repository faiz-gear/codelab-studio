import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        paper: "#FFF8E8",
        "paper-bright": "#FFFDF7",
        "paper-tint": "#F5EDD8",
        ink: "#111111",
        "ink-soft": "#4A4439",
        "hl-yellow": "#FFE84D",
        "hl-pink": "#FF6FAE",
        "hl-teal": "#16A394",
        "hl-teal-deep": "#0B6E63",
        "hl-pink-deep": "#A8185B",
      },
      fontFamily: {
        display: [
          '"Shantell Sans"',
          '"ZCOOL KuaiLe"',
          '"PingFang SC"',
          '"Hiragino Sans GB"',
          '"Noto Sans SC"',
          "sans-serif",
        ],
        sans: [
          '"Onest"',
          '"PingFang SC"',
          '"Hiragino Sans GB"',
          '"Noto Sans SC"',
          "-apple-system",
          "BlinkMacSystemFont",
          "sans-serif",
        ],
        mono: [
          '"JetBrains Mono"',
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Monaco",
          "Consolas",
          "monospace",
        ],
      },
    },
  },
  plugins: [],
} satisfies Config;
