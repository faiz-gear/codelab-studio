---
name: CodeLab Studio
description: Pure-neutral gallery workbench with progressive disclosure, WCAG AA cool pastel semantics, and critically damped motion.
colors:
  carbon-control: "oklch(18% 0 0)"
  graphite-accent: "oklch(33% 0 0)"
  chalk-control-dark: "oklch(94% 0 0)"
  silver-accent-dark: "oklch(80% 0 0)"
  botanical-mint: "oklch(50% 0.12 162)"
  botanical-mint-dark: "oklch(79% 0.14 162)"
  cerulean-mist: "oklch(51% 0.11 240)"
  cerulean-mist-dark: "oklch(79% 0.095 236)"
  studio-violet: "oklch(51% 0.16 290)"
  studio-violet-dark: "oklch(74% 0.125 290)"
  ochre-amber: "oklch(50% 0.12 68)"
  ochre-amber-dark: "oklch(83% 0.12 78)"
  crimson-rose: "oklch(53% 0.18 10)"
  crimson-rose-dark: "oklch(71% 0.155 8)"
  verdigris-teal: "oklch(50% 0.095 182)"
  verdigris-teal-dark: "oklch(79% 0.13 182)"
  gallery-white: "oklch(100% 0 0)"
  paper-muted: "oklch(97.8% 0 0)"
  paper-subtle: "oklch(95.8% 0 0)"
  hairline-subtle: "oklch(96.2% 0 0)"
  hairline: "oklch(93% 0 0)"
  hairline-strong: "oklch(82% 0 0)"
  pewter-muted: "oklch(52% 0 0)"
  graphite-slate: "oklch(46% 0 0)"
  carbon-ink: "oklch(15% 0 0)"
  obsidian-sidebar: "oklch(17.8% 0 0)"
  obsidian-canvas: "oklch(19% 0 0)"
  obsidian-surface: "oklch(21.5% 0 0)"
  obsidian-raised: "oklch(24% 0 0)"
  obsidian-muted: "oklch(25.5% 0 0)"
  obsidian-border-subtle: "oklch(24.5% 0 0)"
  obsidian-border: "oklch(28.5% 0 0)"
  obsidian-border-strong: "oklch(39% 0 0)"
  ash-muted-dark: "oklch(66% 0 0)"
  silver-secondary-dark: "oklch(77% 0 0)"
  chalk-ink-dark: "oklch(96% 0 0)"
typography:
  display:
    fontFamily: '"Geist", -apple-system, BlinkMacSystemFont, "PingFang SC", "Noto Sans SC", "Hiragino Sans GB", sans-serif'
    fontSize: "2.5rem"
    fontWeight: 500
    lineHeight: 1.12
    letterSpacing: "-0.03em"
  headline:
    fontFamily: '"Geist", -apple-system, BlinkMacSystemFont, "PingFang SC", "Noto Sans SC", "Hiragino Sans GB", sans-serif'
    fontSize: "1.4375rem"
    fontWeight: 500
    lineHeight: 1.24
    letterSpacing: "-0.025em"
  title:
    fontFamily: '"Geist", -apple-system, BlinkMacSystemFont, "PingFang SC", "Noto Sans SC", "Hiragino Sans GB", sans-serif'
    fontSize: "1.0625rem"
    fontWeight: 500
    lineHeight: 1.3
    letterSpacing: "-0.02em"
  body:
    fontFamily: '"Inter", -apple-system, BlinkMacSystemFont, "PingFang SC", "Noto Sans SC", "Hiragino Sans GB", sans-serif'
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "-0.01em"
  label:
    fontFamily: '"Inter", -apple-system, BlinkMacSystemFont, "PingFang SC", "Noto Sans SC", "Hiragino Sans GB", sans-serif'
    fontSize: "0.75rem"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "-0.01em"
  code:
    fontFamily: '"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace'
    fontSize: "0.84375rem"
    fontWeight: 400
    lineHeight: 1.63
    letterSpacing: "0em"
    fontFeature: '"tnum" 1'
rounded:
  xs: "6px"
  control: "10px"
  control-lg: "14px"
  panel: "16px"
  card: "18px"
  modal: "20px"
  drawer: "22px"
  surface: "24px"
  pill: "9999px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "5": "18px"
  "6": "20px"
  "8": "24px"
  "10": "32px"
  "12": "40px"
components:
  button-primary:
    backgroundColor: "{colors.carbon-control}"
    textColor: "{colors.gallery-white}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "0 14px"
    height: "34px"
  button-primary-lg:
    backgroundColor: "{colors.carbon-control}"
    textColor: "{colors.gallery-white}"
    typography: "{typography.body}"
    rounded: "{rounded.pill}"
    padding: "0 22px"
    height: "42px"
  button-secondary:
    backgroundColor: "{colors.gallery-white}"
    textColor: "{colors.carbon-ink}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "0 14px"
    height: "34px"
  button-secondary-hover:
    backgroundColor: "{colors.paper-muted}"
    textColor: "{colors.carbon-ink}"
  button-ghost:
    textColor: "{colors.graphite-slate}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "0 14px"
    height: "34px"
  button-danger:
    backgroundColor: "{colors.crimson-rose}"
    textColor: "{colors.gallery-white}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "0 14px"
    height: "34px"
  segmented-control:
    backgroundColor: "{colors.paper-muted}"
    rounded: "{rounded.pill}"
    padding: "3px"
  chip-badge:
    backgroundColor: "{colors.paper-muted}"
    textColor: "{colors.graphite-slate}"
    typography: "{typography.label}"
    rounded: "{rounded.pill}"
    padding: "0 8px"
    height: "22px"
  card-course:
    backgroundColor: "{colors.gallery-white}"
    textColor: "{colors.carbon-ink}"
    rounded: "{rounded.card}"
    padding: "22px 24px"
  input-field:
    backgroundColor: "{colors.gallery-white}"
    textColor: "{colors.carbon-ink}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "0 11px"
    height: "36px"
  topbar:
    backgroundColor: "{colors.gallery-white}"
    textColor: "{colors.carbon-ink}"
    padding: "0 16px"
    height: "48px"
  selection-pill:
    backgroundColor: "{colors.gallery-white}"
    textColor: "{colors.carbon-ink}"
    rounded: "{rounded.pill}"
    padding: "0 5px"
    height: "34px"
  acp-permission-card:
    backgroundColor: "{colors.gallery-white}"
    textColor: "{colors.carbon-ink}"
    rounded: "{rounded.control-lg}"
    padding: "12px 14px"
---

# Design System: CodeLab Studio

## Overview

**Creative North Star: "The Quiet Atelier"**

CodeLab Studio is designed as a calm, gallery-lit engineering bench where generous neutral whitespace frames editorial exercises, live source code, and deterministic verification without visual noise. Because learners spend hours inside the workbench reading technical steps, editing code, and diagnosing terminal output, the interface deliberately steps back: pure achromatic surfaces carry the chrome so syntax highlighting, pass/fail states, and language tags read immediately at a glance.

The system follows progressive disclosure as an architectural discipline rather than an afterthought. Every screen opens to a single clear priority—one centered entry on Welcome Home, two spacious panes in the default Workbench—while secondary instruments (project file tree, auxiliary debug commands, reflection prompts, and the AI/ACP tutor drawer) stay tucked away until summoned. Confirmed visual rejections include heavy multi-chrome IDE toolbars, blue-gray tinted dark modes that muddy syntax colors, and non-actionable decorative stat walls.

**Key Characteristics:**
- Pure achromatic Light (`Gallery White`) and Dark (`Obsidian Canvas`) neutral scales calibrated to WCAG 2.1 AA (≥ 4.5:1) across every foreground and surface pair.
- Monochrome-first default controls backed by 8 user-switchable Arc `data-accent` presets (`neutral`, `violet`, `blue`, `green`, `amber`, `orange`, `coral`, `rose`) tuned separately for Light and Dark themes.
- Cool pastel categorical and status tones rendered through an `oklab` soft-wash formula so badges signal meaning without visual shouting.
- Medium-weight editorial typography (`Geist`) paired with neutral UI sans (`Inter`) and tabular code/metrics (`JetBrains Mono`), animated with critically damped easing and full `prefers-reduced-motion` support.

## Colors

Pure zero-chroma neutrals form the structural canvas in both Light and Dark themes, reserving chromatic ink strictly for syntax highlighting, categorical language pills, live status signals, and optional user-selected accents.

### Primary

- **Carbon Control** (`carbon-control` in Light; **Chalk Control Dark** `chalk-control-dark` in Dark): High-contrast monochrome fill for primary call-to-action buttons, active progress bar fills, and active switch states, achieving 19.67:1 contrast in Light and 15.59:1 in Dark.
- **Graphite Accent** (`graphite-accent` in Light; **Silver Accent Dark** `silver-accent-dark` in Dark): Default focus rings, active pane splitters, editor dirty-file dots, and active text selection washes. When a learner customizes the Appearance drawer (`data-accent`), this accent pair swaps cleanly across 8 separately calibrated Light/Dark presets (`neutral`, `violet #7747ff`, `blue #0562ef`, `green #0a8f5f`, `amber #b86e00`, `orange #c85200`, `coral #cf3d34`, `rose #c52a78`).

### Secondary

- **Botanical Mint** (`botanical-mint` / `botanical-mint-dark`): Verification passes, Exit 0 feedback, live running daemon pulse dots, and diff additions (5.57:1 Light, 9.60:1 Dark).
- **Cerulean Mist** (`cerulean-mist` / `cerulean-mist-dark`): Go language badges, informational run cues, and numeric syntax tokens (5.67:1 Light, 9.21:1 Dark).
- **Studio Violet** (`studio-violet` / `studio-violet-dark`): AI and ACP tutor badges, removable 4D context chips, reflection boxes, and soft AI action buttons (6.13:1 Light, 7.34:1 Dark).
- **Ochre Amber** (`ochre-amber` / `ochre-amber-dark`): JS/TS language badges, external disk-edit conflict banners, port-in-use recovery alerts, and ACP tool-permission approval cards (6.16:1 Light, 10.25:1 Dark).
- **Crimson Rose** (`crimson-rose` / `crimson-rose-dark`): Java language badges, failed assertion items, destructive shell command confirmation actions, and diff deletions (5.83:1 Light, 6.32:1 Dark).

### Tertiary

- **Verdigris Teal** (`verdigris-teal` / `verdigris-teal-dark`): Python language badges and isolated runtime indicators (5.66:1 Light, 9.58:1 Dark).
- **Atmospheric Hero Mesh** (`--arc-hero-bg`): A muted three-point radial wash (blush rose, sky mist, and warm sand over off-white in Light; muted plum, slate, and umber over charcoal in Dark) reserved exclusively for the centered Welcome Home hero panel. Never applied inside the coding workbench.

### Neutral

- **Gallery White** (`gallery-white`): Primary Light background, pane surface, raised overlay ground, and Light primary-button glyph ink.
- **Paper Muted & Paper Subtle** (`paper-muted`, `paper-subtle`): Recessed tracks, segmented control trays, inline code chips, command headers, verification item rows, and progress tracks in Light mode.
- **Hairline Subtle, Hairline & Hairline Strong** (`hairline-subtle`, `hairline`, `hairline-strong`): `1px` structural dividers, pane separators, resting card borders, and hover-reinforced borders in Light mode.
- **Carbon Ink, Graphite Slate & Pewter Muted** (`carbon-ink`, `graphite-slate`, `pewter-muted`): Primary Light reading ink (19.67:1), secondary copy (7.13:1), and AA-calibrated metadata/line-numbers/placeholders (5.42:1).
- **Obsidian Sidebar, Obsidian Canvas, Obsidian Surface, Obsidian Raised & Obsidian Muted** (`obsidian-sidebar`, `obsidian-canvas`, `obsidian-surface`, `obsidian-raised`, `obsidian-muted`): Five-step pure achromatic Dark elevation ramp with zero blue-gray cast.
- **Obsidian Hairlines** (`obsidian-border-subtle`, `obsidian-border`, `obsidian-border-strong`): Dark structural borders and hover border lifts.
- **Chalk Ink Dark, Silver Secondary Dark & Ash Muted Dark** (`chalk-ink-dark`, `silver-secondary-dark`, `ash-muted-dark`): Dark primary ink (15.59:1), secondary prose (8.45:1), and AA-calibrated muted metadata (5.60:1 on surface, 4.95:1 on muted).

### Named Rules

**The Zero-Hue Neutral Rule.** Every background, surface, sidebar, border, and neutral text token uses strictly `0` chroma (`oklch(L% 0 0)`). Blue-gray and warm-gray chrome tints are prohibited so syntax highlighting and status hues stay true.

**The Pastel Tint Formula Rule.** Categorical badges, language tags, context chips, and soft status buttons never use raw saturated fills; they derive their tint via `color-mix(in oklab, var(--tone) var(--badge-tint), var(--surface))` (`9%` Light / `14%` Dark) paired with `var(--badge-ink)` (`86%` Light / `94%` Dark) foreground ink.

**The Monochrome Default Rule.** Out of the box, primary actions, progress fills, and chrome remain monochrome (`carbon-control` / `chalk-control-dark`), keeping chromatic ink reserved for live status and user-opted `data-accent` customization.

## Typography

**Display Font:** Geist (fallback `-apple-system, BlinkMacSystemFont, "PingFang SC", "Noto Sans SC", "Hiragino Sans GB", sans-serif`)
**Body Font:** Inter (fallback `-apple-system, BlinkMacSystemFont, "PingFang SC", "Noto Sans SC", "Hiragino Sans GB", sans-serif`)
**Label/Mono Font:** JetBrains Mono (fallback `ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace`)

**Character:** Editorial restraint meets developer precision—Geist stays at light-to-medium weight (`500`) with tight negative tracking to feel composed rather than loud, Inter carries crisp bilingual (`zh-CN` / `en-US`) readability, and JetBrains Mono anchors code, paths, terminal output, and tabular numbers.

### Hierarchy

- **Display** (`500`, `40px` / `2.5rem`, `1.12` line-height, `-0.03em` tracking): Welcome Home hero heading only, capped at `18ch` measure.
- **Headline** (`500`, `23px` / `1.4375rem`, `1.24` line-height, `-0.025em` tracking): Active exercise chapter title at the top of the guided manual.
- **Title** (`500`, `16.5px–18px` / `1.0625rem` nominal, `1.3` line-height, `-0.02em` tracking): Section heads (`18px`), Appearance drawer (`17px`), course card titles (`16.5px`), modal headers (`16px`), and numbered step headers (`15px`).
- **Body** (`400`, `14px` / `0.875rem`, `1.5–1.65` line-height, `-0.01em` tracking): Manual instructions (`580px` max column width), hero subcopy (`15px`, `44ch` max), card descriptions (`13.5px`), and AI tutor responses (`13.5px`, `1.62` line-height).
- **Label** (`500`, `11.5px–13px` / `0.75rem` nominal, `1.4` line-height, `-0.01em` tracking): Buttons (`12px–13px`), segmented controls (`12px`), badges and context chips (`11.5px`), form labels (`12.5px`), and status footers (`11.5px`).
- **Code / Mono** (`400` / `500`, `13.5px` editor default with `22px` line-height, `12.5px` terminal with `20px` line-height, `11px–12px` inline code / tabs / language tags, `0` tracking, `tabular-nums`): Code editor, terminal viewport, command blocks, diffs, file tabs, `cwd` pills, progress counters, and token counts.

### Named Rules

**The Medium-Ceiling Rule.** Display and UI headings stop at weight `500` (Medium) rather than `700` (Bold). Hierarchy comes from scale, tight negative tracking (`-0.02em` to `-0.03em`), and generous whitespace rather than heavy strokes.

**The Tabular Figures Rule.** All numeric progress ratios (`4/10`), test latencies (`4ms`), line/column readouts, and AI token budgets (`~786 tok`) use `JetBrains Mono` with `font-variant-numeric: tabular-nums` so live updates never jitter horizontally.

## Layout

CodeLab Studio is a desktop-first Tauri 2 workspace engineered for windows `≥ 1024×720` with an optimal canvas at `1440×900`, built on a `4px` spatial unit (`4`, `8`, `12`, `16`, `18`, `20`, `24`, `32`, `40px`).

- **Desktop Chrome:** Fixed `48px` translucent topbar (`padding: 0 16px`, `gap: 10px`) holding at most 6 quiet control groups; non-wrapping across all supported desktop widths.
- **Welcome Home:** Single scrollable column centered at `1000px` max-width (`padding: 36px 40px 72px`, `40px` vertical section gap): centered hero panel (`52px 44px` padding) → `2×2` course library grid (`16px` gap) → quiet single-container recent workspace list.
- **Workbench Default (Two-Pane):** Left guided 3-step manual (`38%` width, `min-width: 340px`, `max-width: 54%`, `28px 32px 60px` reading padding, `580px` inner measure) beside a spacious right code/terminal column (`flex: 1`, `min-width: 360px`) with a compact `212px` bottom terminal/grading panel (`min-height: 115px`, `76%` maximized). Project file tree (`204px`) and AI tutor drawer (`368px`) default to collapsed (`width: 0`).
- **Focus Mode & Splitters:** `⌘/Ctrl + B` collapses the left manual into a `44px` vertical focus rail; `5px` hit-area splitters (displaying a `1px` accent indicator on hover/drag) resize panes smoothly and double-click to reset.
- **Responsive Desktop Breakpoints:**
  - `≥ 1440px`: Opening the AI drawer smoothly reflows the workbench into three panes (`32%` manual / `42%` editor / `26%` AI).
  - `1280px–1360px`: Opening the AI drawer automatically collapses the file tree so the editor never narrows past a comfortable coding width.
  - `< 1280px`: The AI drawer converts into a right-anchored floating overlay (`z-index: 35`) with `--shadow-floating` so the manual and editor widths remain untouched.
  - `≤ 960px`: Course grid and Settings layout collapse cleanly to a single column with horizontal tab navigation.

### Named Rules

**The Two-Pane Default Rule.** Opening a course always starts in a two-pane workbench (3-step manual + editor/terminal). The file tree, secondary commands, reflections, and AI drawer never open unasked on initial load.

**The 520px Editor Floor Rule.** Secondary drawers must yield before the code editor is cramped: auto-collapse the file tree at `≤ 1360px` and float the AI drawer as an overlay below `1280px`.

## Elevation & Depth

Surfaces are flat by default and earn lift strictly through interaction or spatial hierarchy. At rest, workbench panes and cards rely on `1px` achromatic hairlines (`hairline` / `hairline-subtle` in Light, `obsidian-border` / `obsidian-border-subtle` in Dark) rather than drop shadows. Shadows act structurally—separating hovered interactive cards and floating transient surfaces (dropdowns, popovers, modals, slide-over drawers, the selection Ask-AI pill, and centered toasts)—paired with subtle frosted blur (`backdrop-filter: blur(8px–12px)`) on the topbar and modal/drawer scrims.

### Shadow Vocabulary

- **Resting Control** (Light: `0 1px 2px oklch(0% 0 0 / 0.03)` · Dark: `0 1px 2px oklch(0% 0 0 / 0.22)`): Hairline crispness on primary/secondary buttons and active segmented/editor tabs.
- **Raised Card** (Light: `0 8px 24px oklch(0% 0 0 / 0.06), 0 1px 3px oklch(0% 0 0 / 0.03)` · Dark: `0 8px 24px oklch(0% 0 0 / 0.30)`): Hover lift on clickable course cards combined with `translateY(-2px)` and border reinforcement (`hairline-strong`).
- **Floating Overlay** (Light: `0 20px 48px oklch(0% 0 0 / 0.11), 0 3px 10px oklch(0% 0 0 / 0.04)` · Dark: `0 22px 55px oklch(0% 0 0 / 0.48)`): Transient layers crossing pane boundaries—course dropdown, preflight popover, chapter outline drawer, floating selection Ask-AI pill, Appearance drawer, modals, overlay AI drawer (`< 1280px`), and centered toasts.
- **Focus Ring** (`0 0 0 2px var(--background), 0 0 0 4px var(--accent)`): High-visibility double-ring keyboard focus indicator applied universally via `:focus-visible`.

### Named Rules

**The Flat-By-Default Rule.** Panes, command cards, verification lists, and disclosures sit flat at rest with `1px` borders. Shadows appear only on interactive lift or transient floating overlays.

**The Scrim-Only Blur Rule.** Reserve `backdrop-filter: blur()` for the translucent desktop topbar (`12px`) and modal/drawer scrims (`8px`); never blur scrolling editor or manual panes.

## Shapes

CodeLab Studio uses a concentric, sculptural radius ladder that softens outer containers while keeping dense workbench controls crisp and structured.

- **Concentric Radius Scale:** Tiny utility pills and inline code use `6px` (`xs`); workbench inputs, dense buttons, and list items use `10px` (`control`); command blocks, verification cards, ACP permission cards, disclosures, and the AI composer use `14px` (`control-lg`); popovers, dropdowns, and recent lists use `16px` (`panel`); course library cards use `18px` (`card`); modals use `20px` (`modal`); the slide-over Appearance drawer uses `22px` (`drawer`); the Welcome Home hero surface uses `24px` (`surface`).
- **Full Pills (`9999px`):** Reserved for hero primary/secondary CTAs, topbar controls, language tags, status badges, 4D context chips, progress bars, the floating selection Ask-AI pill, and centered notification toasts.
- **Status & Sequence Geometry:** Numbered steps sit inside `22px` circular mono counters; verification items use `18px` circular check/fail badges; live status indicators use `7px` dots (with a `2s` soft ring pulse for active daemon servers); command cards use a `3px` left edge bar to indicate running, passed, or failed state without tinting the whole code surface.

## Components

Components feel refined, restrained, and crisp—quiet monochrome controls with medium-weight Geist headings, soft pastel status cues, and subtle tactile press feedback (`scale(0.975)`), animated with critically damped easing (`cubic-bezier(0.22, 1, 0.36, 1)` for UI transitions, `cubic-bezier(0.16, 1, 0.3, 1)` for entrances, `cubic-bezier(0.32, 0.72, 0, 1)` for slide-over drawers, collapsing under `prefers-reduced-motion`).

### Buttons

- **Shape:** Gently rounded workbench rectangle (`10px` default; `8px` at `xs`, `9px` at `sm`, `9999px` full pill on `lg` hero actions and topbar pills). Heights scale across `26px` (`xs`), `30px` (`sm`), `34px` (`md`), and `42px` (`lg`).
- **Primary:** Solid `carbon-control` background with `gallery-white` text (`chalk-control-dark` on dark ink in Dark mode), `resting` shadow, `0 14px` padding (`0 22px` at `lg`).
- **Hover / Focus / Active:** Primary softens to `opacity: 0.88` on hover; all enabled buttons compress subtly to `scale(0.975)` over `100ms` on press and show the double-ring focus indicator on `:focus-visible`. Disabled state drops to `opacity: 0.42`.
- **Secondary / Ghost / AI-Soft / Danger:** Secondary uses `gallery-white` surface with `1px` `hairline` border, shifting to `paper-muted` + `hairline-strong` on hover. Ghost renders borderless in `graphite-slate` with a `4%` hover wash. AI-Soft and Danger-Soft apply the `oklab` pastel tint recipe (`studio-violet` / `crimson-rose`), while solid Danger (`crimson-rose`) is reserved for confirmed destructive actions.

### Chips

- **Language Tags & Status Badges:** Full pill (`9999px`), `22px` height, `0 8px` padding. Language tags use `JetBrains Mono` (`11px`, `500`) tinted per language (`Go` → `cerulean-mist`, `JS/TS` → `ochre-amber`, `Python` → `verdigris-teal`, `Java` → `crimson-rose`). Status badges use `Inter` (`11.5px`, `500`) across neutral, accent, AI, success, warning, and danger tones.
- **4D Context Chips:** Removable `24px` full pills in the AI composer (`0 7px 0 9px` padding) tinted with `studio-violet` pastel wash and an inline dismiss `×` button (`opacity: 0.7 → 1` on hover).
- **Segmented Controls:** `paper-muted` tray (`3px` padding, `1px` border, `10px` or `9999px` pill radius) with `25px` buttons; active option lifts onto `gallery-white` (`obsidian-surface` in Dark) with `1px` border and `resting` shadow.

### Cards / Containers

- **Course Cards:** `18px` radius, `22px 24px` padding, `gallery-white` (`obsidian-surface` Dark) ground, `1px` `hairline` border, flat at rest, lifting `translateY(-2px)` with `raised` shadow and `hairline-strong` border on hover.
- **Command & Verification Cards:** `14px` radius, `1px` border, flat surface. Command cards pair a `38px` non-wrapping `paper-muted` header (single-line truncated `cwd` pill + run/copy actions) with a `3px` left state stripe and monospace body. Verification cards group `10px`-rounded check rows that expand into two-column monospace `Expected vs Actual` diffs on failure.
- **Progressive Disclosures (`<details>`):** `14px` radius, `paper-muted` header (`12px 15px`), opening to a `gallery-white` body (`14px 15px`) for secondary debug commands and reflection prompts.
- **Modals & Appearance Drawer:** Modals use `20px` radius (`480px` / `720px` / `840px` widths, `86vh` max height) with separate header, scrolling body, and `paper-muted` action footer. The slide-over Appearance drawer uses `22px` radius (`350px` width, `12px` inset margin) with `28px` circular accent swatches.

### Inputs / Fields

- **Style:** `36px` height, `0 11px` padding (`9px 11px` for textareas), `10px` radius, `1px` `hairline` stroke on `gallery-white` (`obsidian-surface` Dark), `13px` body type.
- **Focus:** Border sharpens to `carbon-ink` (`chalk-ink-dark`) with a `3px` `--accent-subtle` halo (`box-shadow: 0 0 0 3px var(--accent-subtle)`).
- **AI Composer:** `14px` container resting on `paper-muted`, shifting to crisp `gallery-white` with `hairline-strong` border and `resting` shadow on `:focus-within`, pairing a borderless auto-growing textarea with a quiet footer bar.

### Navigation

- **Minimalist Desktop Topbar:** `48px` height, `94%` translucent background with `12px` blur and `hairline-subtle` bottom border. Holds window dots (muted at rest, semantic red/amber/green only on hover), brand mark (`24px` inverted square with `7px` radius), contextual course dropdown pill (`30px` height, `240px` max width) + mono progress counter, environment preflight pill, focus toggle (`⌘B`), AI tutor pill, Appearance button, and Settings icon.
- **Editor & Bottom Tab Bars:** `38px` height over `--sidebar-fill` (`2%` tint in Light, `obsidian-sidebar` in Dark); tabs sit as `26px–28px` `8px`-rounded chips that become crisp bordered `gallery-white` pills when active.
- **Chapter Outline Drawer:** Floating `272px` panel (`16px` radius, `floating` shadow) anchored `10px` inside the left manual edge with `10px`-rounded chapter rows.

### Signature Workbench Components

- **Floating Selection Ask-AI Pill:** Fixed `34px` full pill (`9999px`) floating near any `≥ 2`-character text selection in the manual, editor, or terminal (`surface-raised`, `hairline-strong` border, `floating` shadow) with `26px` pill actions separated by `14px` vertical hairlines.
- **ACP Permission Approval Card:** Inline `14px`-rounded governance card inside the AI stream tinted with a `9%` `ochre-amber` wash and `28%` amber border, presenting explicit Allow/Deny actions before an external agent reads, writes, or runs commands; settles to neutral `paper-muted` once resolved.
- **Centered Floating Toasts:** Top-center stack (`top: 58px`, `9999px` pill, `floating` shadow, max 2 visible) keeping notifications completely clear of the AI composer and modal footers.

## Do's and Don'ts

### Do:

- **Do** keep all neutral backgrounds, borders, and chrome strictly zero-chroma (`oklch(L% 0 0)`) in both Light and Dark themes.
- **Do** use the `oklab` soft-tint recipe (`9%` Light / `14%` Dark) for every language tag, status badge, context chip, and soft semantic button so every state clears WCAG 2.1 AA (≥ 4.5:1).
- **Do** keep command headers single-line (`flex-wrap: nowrap`) with truncated `cwd` pills (`max-width: 130px`) so narrow manual panes never wrap action buttons.
- **Do** anchor transient toast notifications at top-center (`top: 58px`, max 2 visible) so they never overlap the bottom-right AI composer or dialog action bars.
- **Do** honor `prefers-reduced-motion: reduce` by collapsing all transition and animation durations to `0ms` / `0.01ms`.

### Don't:

- **Don't** introduce blue-gray or tinted dark-mode surfaces; Dark mode stays strictly pure achromatic Obsidian (`obsidian-canvas` through `obsidian-muted`).
- **Don't** place the Atmospheric Hero Mesh (`--arc-hero-bg`) inside the coding workbench, editor, or drawers; keep it exclusive to the Welcome Home hero panel.
- **Don't** open the file tree, auxiliary debug commands, reflection box, or AI tutor drawer by default when entering a workbench exercise.
- **Don't** add heavy bold (`700`) display weights or non-actionable dashboard statistic walls that compete with the single primary entry or 3-step manual flow.
- **Don't** apply drop shadows to resting workbench panes, command blocks, or verification cards; rely on `1px` hairlines at rest.
