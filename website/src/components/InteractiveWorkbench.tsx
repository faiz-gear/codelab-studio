import { useEffect, useState } from "react";
import {
  COPY,
  GO_EX1_RAW_MARKDOWN,
  GO_EX1_SOLUTION_CODE,
  GO_EX1_STARTER_CODE,
  type Locale,
} from "../data/content";
import {
  ArrowRightIcon,
  CheckIcon,
  CodeFileIcon,
  CrossIcon,
  DiffIcon,
  PlayIcon,
  RefreshIcon,
  TerminalIcon,
  TransformArrowSvg,
} from "./Icons";

interface InteractiveWorkbenchProps {
  locale: Locale;
  mode: "static" | "live";
  onModeChange: (mode: "static" | "live") => void;
  passed: boolean;
  onPassChange: (passed: boolean) => void;
}

function isRouteFixed(source: string): boolean {
  return /HandleFunc\(\s*"GET\s+\/healthz"/.test(source);
}

export function InteractiveWorkbench({
  locale,
  mode,
  onModeChange,
  passed,
  onPassChange,
}: InteractiveWorkbenchProps) {
  const t = COPY[locale].playground;
  const rawMd = GO_EX1_RAW_MARKDOWN[locale];

  const [code, setCode] = useState(GO_EX1_STARTER_CODE[locale]);
  const [hasVerifiedOnce, setHasVerifiedOnce] = useState(true);
  const [verifying, setVerifying] = useState(false);
  const [lastCheckPassed, setLastCheckPassed] = useState(passed);
  const [showDiff, setShowDiff] = useState(false);
  const [motionKey, setMotionKey] = useState(0);

  // Keep comments localized when switching zh-CN <-> en-US while preserving pass/starter state
  useEffect(() => {
    setCode((prev) =>
      isRouteFixed(prev) ? GO_EX1_SOLUTION_CODE[locale] : GO_EX1_STARTER_CODE[locale]
    );
  }, [locale]);

  // Sync external toggle from hero mechanism plate
  useEffect(() => {
    setLastCheckPassed(passed);
    setCode(passed ? GO_EX1_SOLUTION_CODE[locale] : GO_EX1_STARTER_CODE[locale]);
    setMotionKey((k) => k + 1);
  }, [passed, locale]);

  const handleModeSwitch = (nextMode: "static" | "live") => {
    onModeChange(nextMode);
    setMotionKey((k) => k + 1);
  };

  const handleRunVerify = () => {
    setVerifying(true);
    window.setTimeout(() => {
      const ok = isRouteFixed(code);
      setHasVerifiedOnce(true);
      setLastCheckPassed(ok);
      setMotionKey((k) => k + 1);
      if (ok) {
        onPassChange(true);
      }
      setVerifying(false);
    }, 240);
  };

  const handleApplySolution = () => {
    setCode(GO_EX1_SOLUTION_CODE[locale]);
    setShowDiff(false);
    setVerifying(true);
    window.setTimeout(() => {
      setHasVerifiedOnce(true);
      setLastCheckPassed(true);
      setMotionKey((k) => k + 1);
      onPassChange(true);
      setVerifying(false);
    }, 220);
  };

  const handleReset = () => {
    setCode(GO_EX1_STARTER_CODE[locale]);
    setShowDiff(false);
    setHasVerifiedOnce(true);
    setLastCheckPassed(false);
    setMotionKey((k) => k + 1);
    onPassChange(false);
  };

  const passedCount = lastCheckPassed ? 3 : 2;

  return (
    <section id="try-it" className="scroll-mt-20 py-14 lg:py-20">
      <div className="mb-7 flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
        <div className="max-w-2xl">
          <h2 className="font-marker text-3xl leading-tight tracking-[-0.015em] text-ink sm:text-4xl">
            {t.heading}
          </h2>
          <p className="mt-3 text-base leading-relaxed text-ink-soft">
            {t.lead}
          </p>
        </div>
        <span className="inline-flex w-fit items-center gap-2 rounded-full border-2 border-ink bg-paper-tint px-3.5 py-1.5 font-mono text-xs font-medium text-ink">
          <TerminalIcon size={14} className="shrink-0 text-hl-teal-deep" />
          <span>{t.previewBadge}</span>
        </span>
      </div>

      {/* Single focal zine plate with halftone dot-shadow mat; zero nested bordered cards inside */}
      <div className="zine-plate-focal">
        <div className="zine-plate-focal-inner">
          {/* Mode Switcher Bar with toggle-triggered SVG draw-on arrow */}
        <div className="flex flex-wrap items-center justify-between gap-3 border-b-2 border-ink bg-paper-tint px-4 py-3 sm:px-6">
          <div className="flex flex-wrap items-center gap-3">
            <div
              role="tablist"
              aria-label={locale === "zh-CN" ? "工作台视图切换" : "Workbench mode switcher"}
              className="inline-flex rounded-[10px] border-2 border-ink bg-paper-bright p-1"
            >
              <button
                type="button"
                role="tab"
                aria-selected={mode === "static"}
                onClick={() => handleModeSwitch("static")}
                className={`rounded-[7px] px-3.5 py-1.5 text-sm font-bold transition-colors ${
                  mode === "static"
                    ? "bg-hl-pink/40 text-ink"
                    : "text-ink-soft hover:text-ink"
                }`}
              >
                {t.tabStatic}
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={mode === "live"}
                onClick={() => handleModeSwitch("live")}
                className={`rounded-[7px] px-3.5 py-1.5 text-sm font-bold transition-colors ${
                  mode === "live"
                    ? "bg-hl-yellow text-ink"
                    : "text-ink-soft hover:text-ink"
                }`}
              >
                {t.tabLive}
              </button>
            </div>

            <TransformArrowSvg
              key={`arrow-${mode}-${motionKey}`}
              color={mode === "live" ? "#0B6E63" : "#A8185B"}
              className="hidden sm:block"
            />
          </div>

          <div className="flex flex-wrap items-center gap-3 font-mono text-xs">
            {passed && (
              <span
                key={`sticky-${motionKey}`}
                className="zine-swipe-reveal inline-flex items-center gap-1.5 rounded-full bg-hl-teal/20 px-3 py-1 font-bold text-hl-teal-deep"
              >
                <CheckIcon size={13} />
                {t.stickyDoneBadge}
              </span>
            )}
            <span className="text-ink-soft tabular">
              go-goframe-practice · ex-1
            </span>
          </div>
        </div>

        {mode === "static" ? (
          <div className="grid grid-cols-1 lg:grid-cols-12">
            <div className="overflow-x-auto border-b-2 border-ink bg-paper-bright p-5 sm:p-7 lg:col-span-8 lg:border-b-0 lg:border-r-2">
              <div className="mb-3 flex items-center justify-between font-mono text-xs font-semibold text-ink-soft">
                <span>builtin-courses/go-goframe-practice/README.md</span>
                <span>Markdown (Read-Only)</span>
              </div>
              <pre className="whitespace-pre-wrap font-mono text-[13px] leading-relaxed text-ink">
                {rawMd}
              </pre>
            </div>

            <div className="flex flex-col justify-between gap-6 bg-paper p-6 sm:p-7 lg:col-span-4">
              <div>
                <p className="font-marker text-xl text-hl-pink-deep">
                  {t.staticPainLabel}
                </p>
                <p className="mt-3 text-[15px] leading-relaxed text-ink">
                  {t.staticPainText}
                </p>
              </div>

              <button
                type="button"
                onClick={() => handleModeSwitch("live")}
                className="zine-btn-primary w-full px-5 py-3.5 text-base"
              >
                <span>{t.transformBtn}</span>
                <ArrowRightIcon size={17} />
              </button>
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-12">
            {/* Left Pane: 3-Step Guided Manual + Verification Checklist (open hairlines, zero nested bordered cards) */}
            <div className="flex flex-col justify-between border-b-2 border-ink bg-paper-bright p-5 sm:p-6 lg:col-span-5 lg:border-b-0 lg:border-r-2">
              <div className="space-y-5">
                {/* Step 1 */}
                <div className="flex items-start gap-3">
                  <span className="mt-0.5 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-hl-teal-deep font-mono text-xs font-bold text-white">
                    1
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <h3 className="text-[15px] font-bold text-ink">{t.step1Title}</h3>
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-hl-teal/20 px-2.5 py-0.5 font-mono text-[11px] font-bold text-hl-teal-deep">
                        <span className="h-2 w-2 rounded-full bg-hl-teal-deep" />
                        go run . · :8080
                      </span>
                    </div>
                    <p className="mt-1 text-xs leading-relaxed text-ink-soft">
                      {t.step1Sub}
                    </p>
                  </div>
                </div>

                {/* Step 2 */}
                <div className="flex items-start gap-3 border-t border-ink/20 pt-4">
                  <span className="mt-0.5 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-hl-yellow font-mono text-xs font-bold text-ink">
                    2
                  </span>
                  <div className="min-w-0 flex-1">
                    <h3 className="text-[15px] font-bold text-ink">{t.step2Title}</h3>
                    <p className="mt-1 text-xs leading-relaxed text-ink-soft">
                      {t.step2Sub}
                    </p>
                    <div className="mt-2.5 flex flex-wrap gap-2">
                      <button
                        type="button"
                        onClick={() => setShowDiff((prev) => !prev)}
                        className="zine-btn-secondary px-3 py-1.5 text-xs"
                      >
                        <DiffIcon size={14} />
                        {showDiff ? t.closeDiffBtn : t.diffBtn}
                      </button>
                      <button
                        type="button"
                        onClick={handleReset}
                        className="inline-flex items-center gap-1.5 rounded-[9px] px-2.5 py-1.5 text-xs font-semibold text-ink-soft transition-colors hover:bg-paper-tint hover:text-ink"
                      >
                        <RefreshIcon size={13} />
                        {t.resetBtn}
                      </button>
                    </div>
                  </div>
                </div>

                {/* Step 3: Verification Checklist (flat tonal rows with ink dividers, zero nested cards) */}
                <div className="flex items-start gap-3 border-t border-ink/20 pt-4">
                  <span className="mt-0.5 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-hl-pink/45 font-mono text-xs font-bold text-ink">
                    3
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="text-[15px] font-bold text-ink">
                        {t.step3Title}{" "}
                        <span className="font-mono text-xs text-ink-soft tabular">
                          {passedCount}/3
                        </span>
                      </h3>
                      <button
                        type="button"
                        onClick={handleRunVerify}
                        disabled={verifying}
                        className="zine-btn-primary shrink-0 whitespace-nowrap px-3.5 py-1.5 text-xs disabled:opacity-60"
                      >
                        <PlayIcon size={13} />
                        {verifying ? t.runningBtn : t.runBtn}
                      </button>
                    </div>

                    <div
                      className="mt-3 divide-y divide-ink/15 border-y border-ink/20"
                      aria-live="polite"
                    >
                      {t.checks.map((chk) => {
                        const isFail =
                          hasVerifiedOnce && !lastCheckPassed && chk.id === "v2";
                        return (
                          <div
                            key={chk.id}
                            className={`px-2.5 py-2.5 text-xs transition-colors ${
                              isFail ? "bg-hl-pink/15" : "bg-transparent"
                            }`}
                          >
                            <div className="flex items-center justify-between gap-2">
                              <div className="flex items-center gap-2 font-mono font-semibold text-ink">
                                <span
                                  className={`inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full ${
                                    isFail
                                      ? "bg-hl-pink text-ink"
                                      : "bg-hl-teal-deep text-white"
                                  }`}
                                >
                                  {isFail ? <CrossIcon size={11} /> : <CheckIcon size={11} />}
                                </span>
                                <span className="break-all">{chk.label}</span>
                              </div>
                              <span className="shrink-0 font-mono text-[11px] text-ink-soft tabular">
                                {chk.ms}
                              </span>
                            </div>

                            {isFail && "expected" in chk && (
                              <div className="mt-2 grid grid-cols-1 gap-2 pl-7 font-mono text-[11.5px] sm:grid-cols-2">
                                <div className="rounded bg-paper-bright px-2.5 py-1.5">
                                  <span className="block text-[10px] font-bold uppercase text-ink-soft">
                                    {t.expectedLabel}
                                  </span>
                                  <span className="font-semibold text-hl-teal-deep">
                                    {chk.expected}
                                  </span>
                                </div>
                                <div className="rounded bg-paper-bright px-2.5 py-1.5">
                                  <span className="block text-[10px] font-bold uppercase text-ink-soft">
                                    {t.actualLabel}
                                  </span>
                                  <span className="font-semibold text-hl-pink-deep">
                                    {chk.actualFail}
                                  </span>
                                </div>
                              </div>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  </div>
                </div>
              </div>

              <p
                key={`hint-${lastCheckPassed}-${motionKey}`}
                className={`mt-5 rounded-[8px] px-3.5 py-2.5 text-xs leading-relaxed ${
                  lastCheckPassed
                    ? "zine-swipe-reveal bg-hl-teal/20 font-semibold text-hl-teal-deep"
                    : "bg-paper text-ink-soft"
                }`}
              >
                {lastCheckPassed ? t.hintAfterPass : t.hintBeforePass}
              </p>
            </div>

            {/* Right Pane: Editable Code Workspace & Optional Side-by-Side Reference Diff */}
            <div className="flex flex-col bg-paper lg:col-span-7">
              <div className="flex flex-wrap items-center justify-between gap-2 border-b-2 border-ink bg-paper-tint px-4 py-2.5 font-mono text-xs">
                <div className="flex items-center gap-2 font-bold text-ink">
                  <CodeFileIcon size={15} className="text-hl-teal-deep" />
                  <span>task-api/main.go</span>
                  {code !== GO_EX1_STARTER_CODE[locale] && (
                    <span className="rounded-full bg-hl-yellow px-2 py-0.5 text-[11px] text-ink">
                      {locale === "zh-CN" ? "已编辑" : "edited"}
                    </span>
                  )}
                </div>
                <span className="text-ink-soft">
                  {locale === "zh-CN"
                    ? "可直接修改第 18 行代码"
                    : "Editable — try changing line 18"}
                </span>
              </div>

              {showDiff ? (
                <div className="flex flex-1 flex-col justify-between">
                  <div className="grid flex-1 grid-cols-1 divide-y-2 divide-ink/20 md:grid-cols-2 md:divide-x-2 md:divide-y-0">
                    <div className="bg-paper-bright p-5">
                      <div className="mb-2 font-mono text-[11px] font-bold text-hl-pink-deep">
                        {t.diffBeforeTitle}
                      </div>
                      <pre className="overflow-x-auto font-mono text-xs leading-relaxed text-ink">
{`func routes() *http.ServeMux {
  mux := http.NewServeMux()
- mux.HandleFunc("/healthz", healthz)
  return mux
}`}
                      </pre>
                    </div>
                    <div className="bg-paper p-5">
                      <div className="mb-2 font-mono text-[11px] font-bold text-hl-teal-deep">
                        {t.diffAfterTitle}
                      </div>
                      <pre className="overflow-x-auto font-mono text-xs leading-relaxed text-ink">
{`func routes() *http.ServeMux {
  mux := http.NewServeMux()
+ mux.HandleFunc("GET /healthz", healthz)
  return mux
}`}
                      </pre>
                    </div>
                  </div>

                  <div className="flex flex-wrap items-center justify-end gap-3 border-t-2 border-ink bg-paper-tint px-5 py-3.5">
                    <button
                      type="button"
                      onClick={() => setShowDiff(false)}
                      className="zine-btn-secondary px-4 py-2 text-xs"
                    >
                      {t.closeDiffBtn}
                    </button>
                    <button
                      type="button"
                      onClick={handleApplySolution}
                      className="zine-btn-primary px-4 py-2 text-xs"
                    >
                      <CheckIcon size={14} />
                      {t.applyDiffBtn}
                    </button>
                  </div>
                </div>
              ) : (
                <div className="relative flex flex-1 flex-col">
                  <div className="flex flex-1 overflow-x-auto bg-paper-bright">
                    <div
                      aria-hidden="true"
                      className="select-none border-r border-ink/15 bg-paper px-3 py-4 text-right font-mono text-xs leading-[1.7] text-ink-soft tabular"
                    >
                      {code.split("\n").map((_, i) => (
                        <div key={i}>{i + 1}</div>
                      ))}
                    </div>
                    <textarea
                      aria-label={locale === "zh-CN" ? "编辑 task-api/main.go" : "Edit task-api/main.go"}
                      value={code}
                      onChange={(e) => setCode(e.target.value)}
                      spellCheck={false}
                      wrap="off"
                      rows={20}
                      className="zine-code-textarea min-h-[380px] w-full flex-1 overflow-x-auto whitespace-pre bg-transparent px-4 py-4 text-[13px] leading-[1.7] text-ink"
                    />
                  </div>

                  {/* Compact Terminal Output Strip */}
                  <div className="border-t-2 border-ink bg-ink px-4 py-3 font-mono text-xs text-paper">
                    <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-paper/75">
                      <span className="inline-flex items-center gap-2">
                        <span className="h-2 w-2 rounded-full bg-hl-teal" />
                        {locale === "zh-CN"
                          ? "受管终端 · task-api (go run .)"
                          : "Managed Daemon · task-api (go run .)"}
                      </span>
                      <span className="tabular">
                        {lastCheckPassed
                          ? "PASS · 3/3 (9ms)"
                          : "FAIL · 2/3 (POST /healthz -> 200)"}
                      </span>
                    </div>
                    <div className="mt-1.5 text-paper/95">
                      {lastCheckPassed
                        ? "$ curl -i -X POST http://127.0.0.1:8080/healthz  =>  HTTP/1.1 405 Method Not Allowed"
                        : "$ curl -i -X POST http://127.0.0.1:8080/healthz  =>  HTTP/1.1 200 OK  # expected 405"}
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
        </div>
      </div>
    </section>
  );
}
