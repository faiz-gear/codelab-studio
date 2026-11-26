import { useState } from "react";
import { COPY, type Locale } from "../data/content";
import { CheckIcon, CrossIcon, RefreshIcon, ShieldIcon, SparkleIcon } from "./Icons";

interface TutorPreviewProps {
  locale: Locale;
}

const ACP_PRESETS = [
  { name: "Claude Code", cmd: "claude --acp" },
  { name: "Gemini CLI", cmd: "gemini --acp" },
  { name: "Codex CLI", cmd: "codex --acp" },
  { name: "OpenCode", cmd: "opencode acp" },
];

export function TutorPreview({ locale }: TutorPreviewProps) {
  const t = COPY[locale].tutorSection;
  const [removedChips, setRemovedChips] = useState<string[]>([]);
  const [tutorMode, setTutorMode] = useState<"socratic" | "direct">("socratic");
  const [permState, setPermState] = useState<"pending" | "allowed" | "denied">("pending");

  const activeChips = t.chips.filter((c) => !removedChips.includes(c.id));
  const usedTokens = activeChips.reduce((sum, c) => sum + c.tokens, 0);
  const budgetPct = Math.min(100, Math.round((usedTokens / 6000) * 100));

  const toggleRemoveChip = (id: string) => {
    setRemovedChips((prev) => [...prev, id]);
  };

  return (
    <section id="tutor" className="scroll-mt-20 border-t-2 border-ink py-14 lg:py-20">
      <div className="max-w-2xl">
        <h2 className="font-marker text-3xl leading-tight tracking-[-0.015em] text-ink sm:text-4xl">
          {t.heading}
        </h2>
        <p className="mt-3 text-base leading-relaxed text-ink-soft">
          {t.lead}
        </p>
      </div>

      {/* Built-in local ACP CLI strip */}
      <div className="mt-7 flex flex-wrap items-center gap-2.5">
        <span className="mr-1 text-xs font-bold text-ink-soft">{t.agentsLabel}:</span>
        {ACP_PRESETS.map((agent) => (
          <div
            key={agent.name}
            className="inline-flex items-center gap-2 rounded-full border-2 border-ink bg-paper-bright px-3.5 py-1.5 text-xs"
          >
            <span className="font-bold text-ink">{agent.name}</span>
            <code className="font-mono text-[11px] text-hl-teal-deep">{agent.cmd}</code>
          </div>
        ))}
      </div>

      {/* Open dot-grid notebook two-column spread (NO outer box wrapping inner bordered cards) */}
      <div className="mt-9 grid grid-cols-1 gap-8 border-t-2 border-ink/25 pt-8 lg:grid-cols-12 lg:gap-10">
        {/* Left: Open Context Snapshot Budget & Removable Chips */}
        <div className="flex flex-col justify-between gap-6 lg:col-span-5 lg:border-r-2 lg:border-ink/20 lg:pr-8">
          <div>
            <div className="flex items-center justify-between gap-2">
              <h3 className="font-marker text-xl text-ink">
                {t.contextTitle}
              </h3>
              <span className="shrink-0 font-mono text-xs font-bold text-hl-teal-deep tabular">
                ~{usedTokens} / 6,000 tok
              </span>
            </div>
            <p className="mt-2 text-sm leading-relaxed text-ink-soft">
              {t.contextSub}
            </p>

            {/* Token budget bar */}
            <div
              className="mt-4 h-2.5 w-full overflow-hidden rounded-full border-2 border-ink bg-paper-bright"
              aria-hidden="true"
            >
              <div
                className="h-full bg-hl-teal-deep transition-all duration-300"
                style={{ width: `${Math.max(4, budgetPct)}%` }}
              />
            </div>

            {/* Removable context chips */}
            <div className="mt-4 flex flex-wrap gap-2">
              {activeChips.map((chip) => (
                <button
                  key={chip.id}
                  type="button"
                  onClick={() => toggleRemoveChip(chip.id)}
                  className="inline-flex items-center gap-2 rounded-full border-2 border-ink bg-paper-bright px-3 py-1.5 text-xs font-semibold text-ink transition-colors hover:bg-hl-pink/25"
                  title={locale === "zh-CN" ? "点击移除该上下文" : "Click to remove context chip"}
                >
                  <span>{chip.label}</span>
                  <span className="font-mono text-[10px] text-ink-soft tabular">
                    {chip.tokens}t
                  </span>
                  <CrossIcon size={11} />
                </button>
              ))}
              {removedChips.length > 0 && (
                <button
                  type="button"
                  onClick={() => setRemovedChips([])}
                  className="inline-flex items-center gap-1.5 rounded-full border-2 border-dashed border-ink px-3 py-1.5 text-xs font-bold text-hl-teal-deep hover:bg-hl-teal/15"
                >
                  <RefreshIcon size={12} />
                  {t.restoreChips}
                </button>
              )}
            </div>
          </div>

          <div className="flex items-center gap-2 border-t border-ink/20 pt-3 font-mono text-xs font-medium text-ink">
            <ShieldIcon size={15} className="shrink-0 text-hl-teal-deep" />
            <span>{t.filterNote}</span>
          </div>
        </div>

        {/* Right: Socratic vs Direct Response + Standalone Focal Permission Approval Card */}
        <div className="flex flex-col justify-between gap-6 lg:col-span-7">
          <div>
            <div className="flex flex-wrap items-center justify-between gap-3">
              <span className="zine-ink-tape">
                <SparkleIcon size={13} />
                <span>ACP · Claude Code</span>
              </span>
              <div
                role="tablist"
                aria-label={locale === "zh-CN" ? "助教风格切换" : "Tutor teaching style"}
                className="inline-flex rounded-[9px] border-2 border-ink bg-paper-bright p-0.5 text-xs"
              >
                <button
                  type="button"
                  role="tab"
                  aria-selected={tutorMode === "socratic"}
                  onClick={() => setTutorMode("socratic")}
                  className={`rounded-[6px] px-3 py-1 font-bold transition-colors ${
                    tutorMode === "socratic"
                      ? "bg-hl-yellow text-ink"
                      : "text-ink-soft hover:text-ink"
                  }`}
                >
                  {t.modeSocratic}
                </button>
                <button
                  type="button"
                  role="tab"
                  aria-selected={tutorMode === "direct"}
                  onClick={() => setTutorMode("direct")}
                  className={`rounded-[6px] px-3 py-1 font-bold transition-colors ${
                    tutorMode === "direct"
                      ? "bg-hl-teal/25 text-ink"
                      : "text-ink-soft hover:text-ink"
                  }`}
                >
                  {t.modeDirect}
                </button>
              </div>
            </div>

            <blockquote className="mt-4 border-y-2 border-ink/20 bg-paper-bright/80 px-4 py-4 text-sm leading-relaxed text-ink">
              {tutorMode === "socratic" ? t.socraticReply : t.directReply}
            </blockquote>
          </div>

          {/* Standalone Focal ACP Permission Approval Plate (un-nested, halftone dot mat cleanly behind opaque inner card) */}
          <div className="zine-plate-focal" aria-live="polite">
            <div
              className={`zine-plate-focal-inner p-5 transition-colors ${
                permState === "pending"
                  ? "!bg-[#FFF3AD]"
                  : permState === "allowed"
                    ? "!bg-[#E3F5F2]"
                    : "!bg-paper-tint"
              }`}
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="inline-flex items-center gap-2 font-mono text-xs font-bold text-ink">
                  <ShieldIcon size={15} className="shrink-0" />
                  {t.permTitle}
                </span>
                {permState !== "pending" && (
                  <button
                    type="button"
                    onClick={() => setPermState("pending")}
                    className="font-mono text-xs font-bold text-hl-teal-deep underline"
                  >
                    {t.permReset}
                  </button>
                )}
              </div>
              <p className="mt-1.5 text-xs leading-relaxed text-ink-soft">
                {t.permDesc}
              </p>

              {permState === "pending" ? (
                <div className="mt-3.5 flex flex-col gap-3 border-t border-ink/20 pt-3.5 sm:flex-row sm:items-center sm:justify-between">
                  <span className="min-w-0 flex-1 font-mono text-xs font-bold text-ink">
                    {t.permRequest}
                  </span>
                  <div className="flex shrink-0 items-center gap-2">
                    <button
                      type="button"
                      onClick={() => setPermState("allowed")}
                      className="inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-[9px] border-2 border-ink bg-ink px-3.5 py-1.5 text-xs font-bold text-paper-bright hover:bg-ink/85"
                    >
                      <CheckIcon size={12} />
                      {t.permAllow}
                    </button>
                    <button
                      type="button"
                      onClick={() => setPermState("denied")}
                      className="zine-btn-secondary shrink-0 whitespace-nowrap px-3.5 py-1.5 text-xs"
                    >
                      {t.permDeny}
                    </button>
                  </div>
                </div>
              ) : (
                <p className="mt-3 border-t border-ink/15 pt-2.5 text-xs font-semibold text-ink">
                  {permState === "allowed" ? t.permAllowedState : t.permDeniedState}
                </p>
              )}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
