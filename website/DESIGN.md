---
name: CodeLab Studio Website
description: Hand-drawn technical zine landing system on uncoated warm notebook paper, crisp 2px ink linework, halftone dot-matrix focal mats, and three flat highlighter inks.
colors:
  marker-yellow: "#FFE84D"
  marker-yellow-hover: "#FFEF78"
  marker-teal-ink: "#0B6E63"
  marker-teal: "#16A394"
  fluorescent-pink: "#FF6FAE"
  fluorescent-pink-ink: "#A8185B"
  notebook-paper: "#FFF8E8"
  bright-card: "#FFFDF7"
  warm-tint: "#F5EDD8"
  printer-ink: "#111111"
  pencil-soft: "#4A4439"
  chalk-white: "#FFFFFF"
typography:
  display:
    fontFamily: '"Shantell Sans", "ZCOOL KuaiLe", "PingFang SC", "Noto Sans SC", sans-serif'
    fontSize: "2.75rem"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  headline:
    fontFamily: '"Shantell Sans", "ZCOOL KuaiLe", "PingFang SC", "Noto Sans SC", sans-serif'
    fontSize: "2.25rem"
    fontWeight: 700
    lineHeight: 1.25
    letterSpacing: "-0.015em"
  title:
    fontFamily: '"Shantell Sans", "ZCOOL KuaiLe", "PingFang SC", "Noto Sans SC", sans-serif'
    fontSize: "1.25rem"
    fontWeight: 700
    lineHeight: 1.3
    letterSpacing: "-0.01em"
  body:
    fontFamily: '"Onest", "PingFang SC", "Hiragino Sans GB", "Noto Sans SC", -apple-system, BlinkMacSystemFont, sans-serif'
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.65
    letterSpacing: "0em"
  label:
    fontFamily: '"Onest", "PingFang SC", "Hiragino Sans GB", "Noto Sans SC", -apple-system, BlinkMacSystemFont, sans-serif'
    fontSize: "0.875rem"
    fontWeight: 700
    lineHeight: 1.35
    letterSpacing: "0em"
  code:
    fontFamily: '"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace'
    fontSize: "0.8125rem"
    fontWeight: 400
    lineHeight: 1.7
    letterSpacing: "0em"
    fontFeature: '"tnum" 1'
rounded:
  swipe: "4px"
  tape: "5px"
  focus: "6px"
  tag: "8px"
  control-sm: "9px"
  control: "11px"
  plate: "14px"
  pill: "9999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  2xl: "32px"
  section-sm: "56px"
  section-lg: "80px"
components:
  button-primary:
    backgroundColor: "{colors.marker-yellow}"
    textColor: "{colors.printer-ink}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "14px 24px"
  button-primary-hover:
    backgroundColor: "{colors.marker-yellow-hover}"
    textColor: "{colors.printer-ink}"
  button-secondary:
    backgroundColor: "{colors.bright-card}"
    textColor: "{colors.printer-ink}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "8px 16px"
  button-secondary-hover:
    backgroundColor: "{colors.warm-tint}"
    textColor: "{colors.printer-ink}"
  ink-tape-badge:
    backgroundColor: "{colors.printer-ink}"
    textColor: "{colors.bright-card}"
    typography: "{typography.code}"
    rounded: "{rounded.tape}"
    padding: "4px 12px"
  plate-resting:
    backgroundColor: "{colors.bright-card}"
    textColor: "{colors.printer-ink}"
    rounded: "{rounded.plate}"
    padding: "16px"
  plate-focal:
    backgroundColor: "{colors.bright-card}"
    textColor: "{colors.printer-ink}"
    rounded: "{rounded.plate}"
    padding: "24px"
  segmented-switcher:
    backgroundColor: "{colors.bright-card}"
    textColor: "{colors.pencil-soft}"
    rounded: "{rounded.control}"
    padding: "4px"
  context-chip:
    backgroundColor: "{colors.bright-card}"
    textColor: "{colors.printer-ink}"
    typography: "{typography.code}"
    rounded: "{rounded.pill}"
    padding: "6px 12px"
  sidebar-rail:
    backgroundColor: "{colors.notebook-paper}"
    textColor: "{colors.printer-ink}"
    width: "248px"
    padding: "28px 20px"
  code-textarea:
    backgroundColor: "{colors.bright-card}"
    textColor: "{colors.printer-ink}"
    typography: "{typography.code}"
    padding: "16px"
---

# Design System: CodeLab Studio Website

## Overview

**Creative North Star: "The Annotated Engineering Zine"**

The CodeLab Studio public website treats developer documentation like a tactile, hand-annotated technical zine laid out on uncoated warm notebook paper. Rather than mimicking a generic dark-mode SaaS pitch deck, every viewport reads like an open engineering workbook: crisp `2px` black printer-ink linework structures the page, a `22px` dot-grid wash grounds the canvas, and three flat highlighter inks—yellow for primary actions and active routes, pink for emphasis swipes and failing diffs, and teal for animated flow connectors and passing checks—guide the eye directly from static Markdown prose into live verifiable code.

Expressive personality stays strictly separated from reading legibility. Hand-lettered marker headings (`Shantell Sans` paired with self-hosted `ZCOOL KuaiLe` for Simplified Chinese) supply zine warmth and margin callouts, while clean geometric sans (`Onest`) carries bilingual explanations and tabular `JetBrains Mono` locks code snippets, check timings, file sizes, and token counts into crisp alignment. Confirmed visual rejections include dark-mode neon-glow landing templates, equal-sized SaaS bento grids, solid black slab drop shadows, and nested bordered cards stacked inside bordered cards.

**Key Characteristics:**
- Uncoated warm paper ground (`#FFF8E8`) washed with a subtle `22px × 22px` dot grid, bright notebook plate surfaces (`#FFFDF7`), and crisp `2px solid #111111` ink linework.
- Three flat highlighter families—Marker Yellow (`#FFE84D`), Marker Teal (`#16A394` strokes / `#0B6E63` AA-safe ink), and Fluorescent Pink (`#FF6FAE` swipes / `#A8185B` AA-safe ink)—strictly paired so every text label clears WCAG 2.1 AA (≥ 4.5:1).
- Halftone dot-matrix offset mats (`5px × 5px` dots offset `7px` down-right) reserved exclusively for primary focal plates, leaving secondary containers flat on `2px` borders and open ink rules.
- Fully self-hosted bilingual typography (`Shantell Sans`, `ZCOOL KuaiLe`, `Onest`, `JetBrains Mono`) with `font-synthesis-weight: none` on marker display text and `tabular-nums` on all numeric diagnostics.
- State-triggered SVG ink motion (`stroke-dashoffset` connector arrows and `clip-path` highlighter swipes) keyed to interactive mode flips and verification passes, honoring `prefers-reduced-motion: reduce`.

## Colors

A warm light-mode print palette anchored in uncoated cream paper and dense printer ink, punctuated strictly by three flat stationery highlighters paired with AA-safe ink variants for text and controls.

### Primary
- **Marker Yellow** (`#FFE84D` / `--zine-yellow`): High-energy stationery highlight reserved for primary download and verification buttons, active navigation and tab pills, inline text selection (`::selection`), the circular arch brand badge, the `"GET /healthz"` route swipe, and active platform tabs.
- **Marker Yellow Hover** (`#FFEF78`): Brightened hover state applied exclusively to `.zine-btn-primary:hover` alongside a `-2px` upward lift.

### Secondary
- **Marker Teal Ink** (`#0B6E63` / `--zine-teal-ink`): AA-safe deep teal used for passing check pills (`3/3` toggle and step circles with white glyphs), `2.5px` keyboard `:focus-visible` outlines, input caret color, curved hero and workbench flow arrows, token progress bars, sidebar offline callout borders, inline external links, and feature icons.
- **Marker Teal** (`#16A394` / `--zine-teal`): Brighter highlighter stroke reserved for decorative SVG marker underlines, secondary CTA bottom rules (`3px`), live daemon indicator dots on dark terminal strips, and translucent 15%–25% tint washes behind Go/Python tags, active welcome/direct tabs, and sticky completion badges.

### Tertiary
- **Fluorescent Pink** (`#FF6FAE` / `--zine-pink`): Bright highlighter wash used for the hero headline marker swipe (at `56%` opacity covering the lower two-thirds of the line), failing verification icons (paired with `#111111` ink), and soft `15%`–`45%` tint washes for static mode tabs, failing check rows, Java language tags, and Step 3 badges.
- **Fluorescent Pink Ink** (`#A8185B` / `--zine-pink-ink`): AA-safe deep magenta-pink reserved for margin callout text, the curved pink pointer arrow, static-mode pain headings, failing diff actual-value readouts, and safety shield icons.

### Neutral
- **Notebook Paper** (`#FFF8E8` / `--zine-paper`): Primary uncoated warm page ground overlaid with a `22px × 22px` radial dot grid (`rgba(17, 17, 17, 0.065)`), also used for secondary split-pane grounds, scrollbar tracks, and foreground text inside the dark terminal output strip.
- **Bright Card** (`#FFFDF7` / `--zine-card`): Crisp warm-white plate surface used for `.zine-plate` and `.zine-plate-focal-inner` cards, secondary buttons, code editor canvas, selectable chips, and inverse text inside black ink-tape badges.
- **Warm Tint** (`#F5EDD8` / `--zine-tint`): Recessed paper tone used for plate header bars, screenshot captions, secondary button hovers, inactive notebook folder tabs, and borderless code preview bands.
- **Printer Ink** (`#111111` / `--zine-ink`): Primary text ink, universal `2px` structural borders, section rules, black ink-tape badges, CTA emphasis ticks, dark managed-terminal output bar, and alpha source for both the canvas dot grid (`6.5%`) and focal halftone mats (`34%`).
- **Pencil Soft** (`#4A4439` / `--zine-ink-soft`): Warm charcoal ink for section leads, body descriptions, inactive navigation links, editor line-number gutters, check latencies, and release metadata.
- **Chalk White** (`#FFFFFF`): Pure high-contrast foreground reserved exclusively for white glyphs and step numerals placed inside solid `#0B6E63` (`Marker Teal Ink`) pills and counters.

### Named Rules
**The AA-Paired Ink Rule.** Bright highlighter fills (`#16A394` Marker Teal and `#FF6FAE` Fluorescent Pink) are restricted to decorative strokes, marker swipes, tint washes (15%–45% opacity), and dark-ink-filled pills; all colored text, icons, focus rings, and white-foreground badges must use their deepened ink twins (`#0B6E63` and `#A8185B`).

**The Three-Highlighter Ceiling Rule.** Keep chromatic accent hues strictly within the yellow, teal, and pink marker triad—yellow for primary actions and focused code/tabs, teal for verification passes and flow connectors, and pink for headline emphasis, failing diffs, and safety callouts—without introducing extra rainbow categories.

## Typography

**Display Font:** `Shantell Sans` (`600`, `700`, `800`) paired with `ZCOOL KuaiLe` (`400`) for Simplified Chinese, falling back to `"PingFang SC"`, `"Noto Sans SC"`, `sans-serif`. All weights are self-hosted via `@fontsource`.
**Body Font:** `Onest` (`400`, `500`, `600`, `700`), falling back to `"PingFang SC"`, `"Hiragino Sans GB"`, `"Noto Sans SC"`, `-apple-system`, `BlinkMacSystemFont`, `sans-serif`.
**Label/Mono Font:** `JetBrains Mono` (`400`, `500`, `700`), falling back to `ui-monospace`, `SFMono-Regular`, `Menlo`, `Monaco`, `Consolas`, `monospace`.

**Character:** Expressive felt-tip marker display headings bring hand-crafted zine energy to bilingual Chinese and English headlines, immediately balanced by upright, crisp `Onest` explanatory prose and precision `JetBrains Mono` source code and telemetry.

### Hierarchy
- **Display** (`700` Latin / native `400` CJK, `2.05rem` mobile → `2.6rem` `sm` → `2.75rem` `xl`, line-height `1.2`, tracking `-0.02em`): Hero `<h1>` thesis statement only; pairs with `.swipe-pink` inline emphasis and a teal SVG `MarkerUnderline`.
- **Headline** (`700` Latin / native `400` CJK, `1.875rem` mobile → `2.25rem` `sm+`, line-height `1.25`, tracking `-0.015em`): Top-level `<h2>` heading for each of the five narrative sections below the hero.
- **Title** (`700` marker or `800` `Onest` for utilitarian platform headers, `1.125rem`–`1.5rem`, line-height `1.3`, tracking `-0.01em`): Card headings, step strip titles, course names, Context Snapshot header, and ink-ruled principle headings.
- **Body** (`400`, `1rem` / `16px` standard, `17px` hero subtitle, `14px`–`15px` dense pane copy, line-height `1.625`–`1.7`, max line measure `54ch`–`65ch`): Lead paragraphs, course summaries, tutor replies, and transparent first-launch notes.
- **Label** (`600`–`700`, `0.75rem`–`0.875rem`, line-height `1.35`, sentence/mixed case): Navigation links, primary and secondary buttons, folder tabs, mode toggles, and interactive context chips.
- **Code** (`400`–`700`, `0.6875rem`–`0.8125rem` / `11px`–`13px`, line-height `1.7`, `font-variant-numeric: tabular-nums`): Editable Go code, raw Markdown source, diffs, terminal output strip, SHA-256 checksum commands, language/runtime pills, and tabular counters.

### Named Rules
**The No-Faux-Bold Marker Rule.** Apply `font-synthesis-weight: none` whenever `.font-marker` pairs multi-weight `Shantell Sans` (`600`–`800`) with single-weight `ZCOOL KuaiLe` (`400`), preventing browser synthetic bolding from blurring Simplified Chinese strokes.

**The Tabular Telemetry Rule.** Every numeric readout that updates or aligns vertically—check ratios (`2/3`, `3/3`), assertion timings (`4ms`), exercise counts (`10 题`), editor line numbers, installer sizes (`16.2 MB`), and ACP token budgets (`~1180 / 6,000 tok`)—uses `JetBrains Mono` with `font-variant-numeric: tabular-nums`.

## Layout

The site uses a two-region editorial shell on desktop (`lg` ≥ `1024px`, `grid-cols-[248px_minmax(0,1fr)]`): a persistent `248px` sticky left zine table-of-contents rail bordered by a `2px solid #111111` vertical rule, beside a fluid notebook canvas whose content centers inside `max-width: 1120px` with progressive horizontal padding (`20px` mobile → `32px` at `640px` → `48px` at `1024px`). Below `1024px`, the sidebar collapses into a single-line sticky top notebook bar (`border-b-2 border-ink`) paired with a fixed dropdown TOC drawer anchored at `top: 54px`.

Vertical section cadence opens with a compact hero (`28px` top padding, `40px`–`48px` bottom padding) and steps cleanly through `56px` (`py-14`) mobile / `80px` (`lg:py-20`) desktop sections separated by full-width `2px solid #111111` horizontal ink rules. Content avoids equal-card grids in favor of deliberate 12-column asymmetric splits calibrated to each section's proof: `6/6` hero thesis beside the vertical README-to-Go flow plate, `8/4` (static) or `5/7` (live) interactive workbench, `7/5` real desktop capture beside ink-ruled principles, `6/6` tabbed course explorer over an open Managed Runtime band, `5/7` open dot-grid ACP tutor spread, and `7/5` installer download plate over an open Course Pack author strip.

Responsive breakpoints follow four active thresholds: `sm` (`640px`) promotes mobile 2×2 course tabs to attached folder tabs and widens gutters; `md` (`768px`) expands the 3-step hero strip into an arrow-linked horizontal sequence (`1fr auto 1fr auto 1fr`) and unlocks side-by-side Reference Diff panes; `lg` (`1024px`) activates the persistent `248px` left rail, desktop top utility bar, and 12-column section spreads; `xl` (`1280px`) activates the 6/6 hero split and `2.75rem` display scale.

### Named Rules
**The Persistent Rail Rule.** Above `1024px`, navigation stays anchored in the `248px` sticky left zine table-of-contents rail while the right top utility strip holds only locale switching, the GitHub link, and the primary download action; below `1024px`, the rail collapses into a single sticky top notebook bar and toggleable dropdown TOC.

**The Open-Paper Spread Rule.** Separate page sections with full-width `2px solid #111111` ink dividers and divide internal plate panes with borderless tonal shifts (`#FFFDF7` / `#F5EDD8` / `#FFF8E8`) and hairlines rather than wrapping bordered cards inside outer bordered cards.

## Elevation & Depth

This system uses tactile print layering rather than blurred drop shadows. The base page is an uncoated dot-grid notebook sheet (`#FFF8E8`); resting plates (`.zine-plate`) sit completely flat at zero elevation, separated solely by `#FFFDF7` fill and a `2px solid #111111` ink border. Primary focal artifacts step forward through a signature halftone dot-matrix offset mat (`::before` pseudo-element on a non-clipping `position: relative; isolation: isolate;` wrapper behind an opaque `.zine-plate-focal-inner` shell) rather than a solid black shadow slab. Conventional box-shadows are excluded from all resting containers and appear exclusively as a subtle ambient lift under hovered primary buttons.

### Shadow & Depth Vocabulary
- **Canvas Dot Grid** (`background-image: radial-gradient(rgba(17, 17, 17, 0.065) 1px, transparent 1px); background-size: 22px 22px`): Continuous notebook texture across `body`.
- **Halftone Focal Mat** (`.zine-plate-focal` with `position: relative; isolation: isolate;` + `::before`: `inset: 7px -7px -7px 7px; border-radius: 14px; background-image: radial-gradient(rgba(17, 17, 17, 0.34) 1.35px, transparent 1.35px); background-size: 5px 5px; z-index: -1; pointer-events: none;`): Print-offset halftone mat rendered cleanly behind an opaque `.zine-plate-focal-inner` (`overflow: hidden`) shell on the hero Go preview card, interactive workbench, desktop capture figure, ACP permission approval card, and download plate.
- **Primary Hover Lift** (`box-shadow: 0 8px 18px rgba(17, 17, 17, 0.12)` + `transform: translateY(-2px)`): Applied exclusively on `.zine-btn-primary:hover`.
- **Keyboard Focus Outline** (`outline: 2.5px solid #0B6E63; outline-offset: 2px; border-radius: 6px`): High-visibility `:focus-visible` indicator across all interactive controls.

### Named Rules
**The Halftone Focal-Only Rule.** Reserve the `7px` offset halftone dot-matrix mat (`.zine-plate-focal`) strictly for primary focal plates (the hero live preview, interactive workbench, desktop capture figure, ACP permission card, and download plate); resting secondary surfaces stay flat with `2px` borders and zero drop shadow.

**The Hover-Only Ambient Shadow Rule.** Avoid solid black slab shadows anywhere in the system; soft ambient elevation (`0 8px 18px rgba(17, 17, 17, 0.12)`) appears solely on `.zine-btn-primary:hover` alongside a `-2px` vertical lift.

## Shapes

Every structural surface uses confident rounded-notebook geometry unified by crisp `2px solid #111111` borders: `14px` corners on resting and focal plates, `11px` on primary and secondary action buttons (stepping down to `9px` on compact header/permission buttons and `6px`–`7px` on inner segmented tab buttons), `8px` on tilted language tags, `5px` on black ink-tape badges, `4px` on inline highlighter swipes, and `9999px` full pills for step counters, status toggles, context chips, and runtime badges.

Physical stationery touches remain restrained and functional. Small movable slips and stickers—the static `README.md` hero card, the four sidebar language quick-jump tags, and the hero pink margin callout—carry a subtle `±1deg` tilt (`-rotate-1` / `rotate-1`) that straightens to `0deg` on hover. Inline `.swipe-pink` emphasis uses a vertical gradient occupying `34%`–`90%` of line height with `box-decoration-break: clone` so multi-line marker strokes wrap cleanly. Hand-drawn vector strokes (`HeroFlowConnector`, `PinkPointerArrow`, `TransformArrowSvg`, `MarkerUnderline`, `SmallCurvedArrow`, `CtaEmphasisTicks`) use `2.2px`–`4.5px` rounded stroke caps (`strokeLinecap="round"`) to match the `2px` ink borders.

### Named Rules
**The Whisper-Tilt Rule.** Physical paper tilt is capped at `±1deg` (`-rotate-1` / `rotate-1`) on small movable artifacts—tilted language jump tags, the static README slip, and margin callout notes—and straightens cleanly to `0deg` on hover; main reading columns and code editors stay strictly upright.

## Components

Every component pairs tactile `2px` ink framing with crisp, instant state feedback (`180ms` exponential deceleration `cubic-bezier(0.16, 1, 0.3, 1)`), self-contained SVG iconography (`2px`–`2.6px` rounded strokes), and full `prefers-reduced-motion` compliance.

### Buttons
Tactile and press-confident stationery controls framed in crisp `2px` ink borders.
- **Shape:** Gently rounded control silhouette (`11px` radius on standard `.zine-btn-primary` / `.zine-btn-secondary`, `9px` on compact mobile/permission controls).
- **Primary:** Marker Yellow fill (`#FFE84D`), Printer Ink label (`#111111`), bold `700` weight, `2px solid #111111` border, `8px` icon gap, `14px 24px` padding on hero/download CTAs (`8px 16px` in top bar, `6px 14px` in compact verify triggers). On hero primary CTA, flanked on desktop (`sm+`) by three radial black ink `CtaEmphasisTicks`.
- **Hover / Focus / Active:** Primary brightens to `#FFEF78`, lifts `translateY(-2px)`, and gains soft ambient shadow (`0 8px 18px rgba(17, 17, 17, 0.12)`); active state depresses to `translateY(0) scale(0.985)`; keyboard `:focus-visible` draws a `2.5px solid #0B6E63` outline offset `2px`.
- **Secondary / Utility:** Bright Card fill (`#FFFDF7`), Printer Ink label (`#111111`), `600` weight, `2px solid #111111` border; hovers to Warm Tint (`#F5EDD8`) with `translateY(-1px)` and zero drop shadow. Quiet ghost reset controls use `#4A4439` text over transparent fill hovering to `#F5EDD8`.
- **Approval Confirm Variant:** High-contrast Printer Ink (`#111111`) fill with Bright Card (`#FFFDF7`) bold label and `2px` border inside the ACP permission approval plate.

### Chips & Tape Badges
Compact tactile labels for language jumps, removable AI context, runtime inventory, and zine annotations.
- **Tilted Language Tags:** `8px` radius, `2px solid #111111` border, `JetBrains Mono` `12px` bold, tinted in `20%–25%` teal (Go/Python), full Marker Yellow (TypeScript), or `40%` pink (Java 21), tilted `±1deg` at rest and straightening to `0deg` on hover.
- **Removable Context Chips & Runtime Pills:** Full pill (`9999px`), `2px solid #111111` border, `#FFFDF7` fill, `6px 12px` padding. Context chips display label, tabular token count (`410t`), and inline cross icon, hovering to `25%` Fluorescent Pink wash to signal removal; a dashed-border pill restores removed chips.
- **Black Ink Tape Badge (`.zine-ink-tape`):** Printer Ink (`#111111`) fill, Bright Card (`#FFFDF7`) marker text (`12px`, `0.03em` tracking), `5px` radius, `4px 12px` padding, used to stamp the active `ACP · Claude Code` channel.

### Cards / Zine Plates
Clear separation between flat resting plates and halftone-backed focal plates, with zero nested bordered boxes inside.
- **Corner Style:** Consistent `14px` radius across both `.zine-plate` and `.zine-plate-focal`.
- **Resting Plate (`.zine-plate`):** `#FFFDF7` fill, `2px solid #111111` border, zero shadow, `16px` internal padding on step cards and static README slips (or split tonal panes inside the tabbed course sheet).
- **Focal Plate (`.zine-plate-focal` + `.zine-plate-focal-inner`):** Non-clipping `position: relative; isolation: isolate;` outer wrapper whose `::before` pseudo-element paints the `7px` down-right halftone dot-matrix mat (`rgba(17, 17, 17, 0.34)` `1.35px` dots on a `5px × 5px` grid, `z-index: -1`), paired with an opaque `.zine-plate-focal-inner` shell (`position: relative; z-index: 1; background-color: var(--zine-card); border: 2px solid var(--zine-ink); border-radius: 14px; overflow: hidden;`) so the halftone mat renders cleanly behind and outside without interior bleed. Used for the hero Go card, interactive workbench, real desktop capture figure, ACP permission approval plate, and download card.
- **Internal Anatomy:** Split panes divide with `2px solid #111111` seam lines and alternate `#FFFDF7`, `#F5EDD8`, and `#FFF8E8` tonal bands; verification checklists and syllabi use subtle `1px` ink dividers (`rgba(17, 17, 17, 0.15–0.20)`) rather than nested borders.

### Inputs / Editable Code Workspace
Direct, low-chrome monospace editing integrated cleanly inside the live workbench plate.
- **Style:** Borderless transparent `.zine-code-textarea` sitting on `#FFFDF7` bright card surface beside a select-none `#FFF8E8` tabular line-number gutter separated by a `1px` `ink/15` hairline. `JetBrains Mono` `13px`, line-height `1.7`, `tab-size: 4`, `min-height: 380px`, `caret-color: #0B6E63`.
- **Footer Strip:** Anchored directly beneath the editor by a `2px solid #111111` top seam: solid `#111111` terminal bar carrying `#FFF8E8` monospace output, live `#16A394` daemon dot, and tabular `PASS · 3/3 (9ms)` / `FAIL · 2/3` readout.

### Navigation
Dual-mode editorial wayfinding that keeps orientation persistent on desktop without crowding mobile viewports.
- **Desktop Sidebar (`≥ 1024px`):** `248px` sticky full-height rail (`bg-paper/95`, `border-r-2 border-ink`, `28px 20px` padding) containing the `36px` circular yellow `BrandMarkIcon` badge (tilts `-6deg` on hover) + `CodeLab Studio` marker wordmark underlined by `MarkerUnderline`, five icon-prefixed scroll-spied section links (active link framed in `2px solid #111111` + `75%` Marker Yellow fill, `10px` radius; inactive links in `#4A4439` hovering to `#F5EDD8`), tilted language quick-jump tags, and a bottom `#0B6E63`-bordered offline note box.
- **Desktop Top Utility Bar (`≥ 1024px`):** Right-aligned strip holding bilingual locale toggle (`English` / `简体中文`), `GitHub` secondary link, and yellow `Download v0.1.0` primary button.
- **Mobile Top Bar & Drawer (`< 1024px`):** Sticky `border-b-2 border-ink` notebook header (`px-3.5 py-2.5`) with compact `32px` brand badge, mono locale switch, yellow download button, and hamburger menu toggle opening a full-width fixed TOC drawer.
- **Segmented Mode Switchers:** `10px` (or `9px`) `2px`-bordered `#FFFDF7` trays housing `6px`–`7px` tab buttons filled with `#FFE84D` (live / workbench / socratic), `40%` `#FF6FAE` (static README), or `25%` `#16A394` (welcome / direct).

### Signature Components

#### Hero Mechanism Flow & SVG Motion Grammar
Pairs a `-1deg` tilted static `README.md` plate above an in-flow animated teal `HeroFlowConnector` arrow (`stroke="#0B6E63"`, `strokeWidth="3"`) pointing cleanly into a halftone-backed `task-api/main.go` focal card with a clickable `2/3 ↔ 3/3` pill and curved `#A8185B` `PinkPointerArrow` margin annotation. Every interactive mode toggle or verification pass increments a React motion key that replays two crisp ink animations under `--zine-ease` (`cubic-bezier(0.16, 1, 0.3, 1)`):
- **SVG Ink Draw-On (`.zine-draw-path`, `.zine-draw-underline`):** `stroke-dashoffset` animates from `280 → 0` over `780ms` on connector arrows and `460 → 0` over `820ms` (`120ms` delay) on marker underlines.
- **Sliding Highlighter Swipe (`.zine-swipe-reveal`):** `clip-path: inset(0 100% 0 0)` wipes left-to-right to `inset(0 0 0 0)` over `420ms` across the focused `"GET /healthz"` token, sticky completion badge, and post-pass feedback bar.
All motion collapses immediately to static final states under `@media (prefers-reduced-motion: reduce)`.

#### Attached Notebook Folder Course Tabs
Four course tabs (`Go`, `TypeScript`, `Python`, `Java 21` with tabular exercise-count pills) arrange into a tight `2×2` grid on mobile (`< 640px`) and merge on `sm+` into top-rounded (`12px`) folder tabs whose active tab pulls down `-2px` (`sm:-mb-[2px] sm:pb-3`) with `border-b-0` to fuse seamlessly into the flat `2px`-bordered single-sheet course plate beneath.

## Do's and Don'ts

### Do:
- **Do** self-host every typeface (`Shantell Sans`, `ZCOOL KuaiLe`, `Onest`, `JetBrains Mono`) via bundled `@fontsource` assets so the bilingual zine renders identically offline and across locales.
- **Do** pair bright highlighters (`#16A394`, `#FF6FAE`) with their AA-safe ink twins (`#0B6E63`, `#A8185B`) on every colored text label, pointer arrow, focus outline (`2.5px solid #0B6E63`), and solid badge carrying white glyphs.
- **Do** use `2px solid #111111` structural borders on plates, controls, and section dividers, and keep internal sub-regions flat with tonal shifts (`#FFFDF7`, `#F5EDD8`, `#FFF8E8`) and light ink hairlines (`rgba(17, 17, 17, 0.15–0.20)`).
- **Do** keep SVG ink connectors (`HeroFlowConnector`, `PinkPointerArrow`, `TransformArrowSvg`) in-flow with state-keyed `stroke-dashoffset` draw-on animations and collapse all motion cleanly under `prefers-reduced-motion: reduce`.
- **Do** preserve `tabular-nums` (`JetBrains Mono`) on every check counter, latency, file size, line gutter, and token budget meter.

### Don't:
- **Don't** replace halftone dot-matrix offset mats (`5px × 5px` radial dots) with solid black neobrutalist slab shadows or heavy blurred dark-mode glows.
- **Don't** nest bordered `.zine-plate` boxes inside another bordered `.zine-plate` or `.zine-plate-focal` container; use tonal bands and hairline dividers inside plates instead.
- **Don't** use raw `#16A394` or `#FF6FAE` for body copy, links, or small labels on warm `#FFF8E8` / `#FFFDF7` paper—they fall below WCAG 2.1 AA text contrast without their deep ink counterparts.
- **Don't** add eyebrow kickers above section headings or decorative equal-card SaaS bento grids; open every section directly with its marker `<h2>` heading and prose lead.
- **Don't** apply synthetic bolding to CJK marker headings or tilt large readable panels beyond `0deg`.
