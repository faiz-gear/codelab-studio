import { useEffect, useState } from "react";
import {
  COPY,
  INSTALLERS,
  RELEASE_TAG_URL,
  RELEASE_VERSION,
  REPO_URL,
  type CourseId,
  type Locale,
  type PlatformId,
} from "./data/content";
import { CoursesExplorer } from "./components/CoursesExplorer";
import { DownloadSection } from "./components/DownloadSection";
import {
  ArrowDownIcon,
  BookIcon,
  BrandMarkIcon,
  CheckIcon,
  CodeFileIcon,
  CrossIcon,
  CtaEmphasisTicks,
  DownloadIcon,
  ExternalIcon,
  FolderIcon,
  GitHubIcon,
  GlobeIcon,
  HeroFlowConnector,
  MarkerUnderline,
  MenuIcon,
  PinkPointerArrow,
  PlayIcon,
  ShieldIcon,
  SmallCurvedArrow,
  SparkleIcon,
} from "./components/Icons";
import { InteractiveWorkbench } from "./components/InteractiveWorkbench";
import { TutorPreview } from "./components/TutorPreview";

function detectPlatform(): PlatformId {
  if (typeof navigator === "undefined") return "macos";
  const ua = `${navigator.userAgent} ${navigator.platform}`.toLowerCase();
  if (ua.includes("win")) return "windows";
  if (ua.includes("linux") && !ua.includes("android")) return "linux";
  return "macos";
}

const SECTION_IDS = ["try-it", "markdown", "courses", "tutor", "download"] as const;

export function App() {
  const [locale, setLocale] = useState<Locale>("zh-CN");
  const [detectedPlatform] = useState<PlatformId>(() => detectPlatform());
  const [selectedPlatform, setSelectedPlatform] = useState<PlatformId>(detectedPlatform);
  const [activeSection, setActiveSection] = useState<string>("try-it");
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  // Interactive state: hero starts on green 3/3 (clickable to toggle 2/3 <-> 3/3), full lab starts at 2/3 ready to solve
  const [workbenchMode, setWorkbenchMode] = useState<"static" | "live">("live");
  const [heroDemoPassed, setHeroDemoPassed] = useState(true);
  const [workbenchPassed, setWorkbenchPassed] = useState(false);
  const [heroMotionKey, setHeroMotionKey] = useState(0);
  const [activeCourse, setActiveCourse] = useState<CourseId>("go");
  const [activeScreenshot, setActiveScreenshot] = useState<"workbench" | "welcome">("workbench");

  const t = COPY[locale];
  const heroInstaller = INSTALLERS[detectedPlatform];

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  useEffect(() => {
    const updateActive = () => {
      const scrollY = window.scrollY + 220;
      let current: string = SECTION_IDS[0];
      for (const id of SECTION_IDS) {
        const el = document.getElementById(id);
        if (el && el.offsetTop <= scrollY) {
          current = id;
        }
      }
      setActiveSection(current);
    };
    updateActive();
    window.addEventListener("scroll", updateActive, { passive: true });
    return () => window.removeEventListener("scroll", updateActive);
  }, []);

  const jumpToCourse = (id: CourseId) => {
    setActiveCourse(id);
    setMobileMenuOpen(false);
    document.getElementById("courses")?.scrollIntoView({ behavior: "smooth" });
  };

  const handleHeroPassToggle = () => {
    setHeroDemoPassed((prev) => !prev);
    setHeroMotionKey((k) => k + 1);
  };

  const navItems = [
    { id: "try-it", label: t.nav.tryIt, icon: <PlayIcon size={15} /> },
    { id: "markdown", label: t.nav.markdown, icon: <BookIcon size={15} /> },
    { id: "courses", label: t.nav.courses, icon: <CodeFileIcon size={15} /> },
    { id: "tutor", label: t.nav.tutor, icon: <SparkleIcon size={15} /> },
    { id: "download", label: t.nav.download, icon: <DownloadIcon size={15} /> },
  ];

  const renderPillarIcon = (kind: "folder" | "shield" | "code") => {
    if (kind === "folder") return <FolderIcon size={18} className="shrink-0 text-hl-teal-deep" />;
    if (kind === "shield") return <ShieldIcon size={18} className="shrink-0 text-hl-pink-deep" />;
    return <CodeFileIcon size={18} className="shrink-0 text-hl-teal-deep" />;
  };

  return (
    <div className="min-h-screen lg:grid lg:grid-cols-[248px_minmax(0,1fr)]">
      {/* Skip link for keyboard navigation */}
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-lg focus:border-2 focus:border-ink focus:bg-hl-yellow focus:px-4 focus:py-2 focus:font-bold focus:text-ink"
      >
        {locale === "zh-CN" ? "跳至正文内容" : "Skip to main content"}
      </a>

      {/* Mobile / Tablet Top Notebook Bar (single-line crisp brand) */}
      <header className="sticky top-0 z-40 flex items-center justify-between gap-2 border-b-2 border-ink bg-paper/95 px-3.5 py-2.5 backdrop-blur-sm lg:hidden">
        <a href="#top" className="flex shrink-0 items-center gap-2">
          <span className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-full border-2 border-ink bg-hl-yellow">
            <BrandMarkIcon size={15} />
          </span>
          <span className="whitespace-nowrap font-marker text-sm font-bold tracking-tight text-ink sm:text-base">
            CodeLab Studio
          </span>
        </a>
        <div className="flex shrink-0 items-center gap-1.5">
          <button
            type="button"
            onClick={() => setLocale(locale === "zh-CN" ? "en-US" : "zh-CN")}
            className="inline-flex items-center gap-1 whitespace-nowrap rounded-[9px] border-2 border-ink bg-paper-bright px-2 py-1.5 font-mono text-xs font-bold text-ink"
          >
            <GlobeIcon size={13} />
            {t.nav.langSwitch}
          </button>
          <a
            href="#download"
            className="zine-btn-primary whitespace-nowrap px-2.5 py-1.5 text-xs"
          >
            {t.nav.downloadShort}
          </a>
          <button
            type="button"
            aria-label={locale === "zh-CN" ? "打开目录菜单" : "Toggle navigation menu"}
            aria-expanded={mobileMenuOpen}
            onClick={() => setMobileMenuOpen((prev) => !prev)}
            className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-[9px] border-2 border-ink bg-paper-bright text-ink"
          >
            {mobileMenuOpen ? <CrossIcon size={17} /> : <MenuIcon size={17} />}
          </button>
        </div>
      </header>

      {/* Mobile Dropdown TOC Drawer */}
      {mobileMenuOpen && (
        <nav
          aria-label={locale === "zh-CN" ? "移动端目录" : "Mobile navigation"}
          className="fixed inset-x-0 top-[54px] z-30 border-b-2 border-ink bg-paper px-5 py-4 lg:hidden"
        >
          <div className="flex flex-col gap-1.5">
            {navItems.map((item) => (
              <a
                key={item.id}
                href={`#${item.id}`}
                onClick={() => setMobileMenuOpen(false)}
                className={`inline-flex items-center gap-2.5 rounded-[10px] px-3 py-2 text-sm font-bold ${
                  activeSection === item.id
                    ? "border-2 border-ink bg-hl-yellow/75 text-ink"
                    : "text-ink-soft"
                }`}
              >
                {item.icon}
                <span>{item.label}</span>
              </a>
            ))}
            <a
              href={REPO_URL}
              target="_blank"
              rel="noreferrer"
              className="mt-2 inline-flex items-center gap-2 rounded-[10px] border-2 border-ink bg-paper-bright px-3 py-2 text-sm font-bold text-ink"
            >
              <GitHubIcon size={15} />
              {t.nav.github}
            </a>
          </div>
        </nav>
      )}

      {/* Desktop Persistent Left Zine TOC Sidebar */}
      <aside className="hidden lg:sticky lg:top-0 lg:flex lg:h-screen lg:flex-col lg:justify-between lg:border-r-2 lg:border-ink lg:bg-paper/95 lg:px-5 lg:py-7">
        <div>
          <a href="#top" className="group block pb-4">
            <div className="flex items-center gap-2.5">
              <span className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full border-2 border-ink bg-hl-yellow transition-transform group-hover:-rotate-6">
                <BrandMarkIcon size={17} />
              </span>
              <span className="font-marker text-lg font-bold tracking-tight text-ink">
                CodeLab Studio
              </span>
            </div>
            <MarkerUnderline className="mt-2 h-2.5 w-full" />
          </a>

          <nav
            aria-label={locale === "zh-CN" ? "页面目录" : "Page sections"}
            className="mt-4 flex flex-col gap-1.5"
          >
            {navItems.map((item) => {
              const isActive = activeSection === item.id;
              return (
                <a
                  key={item.id}
                  href={`#${item.id}`}
                  className={`inline-flex items-center gap-2.5 rounded-[10px] px-3 py-2 text-sm font-bold transition-colors ${
                    isActive
                      ? "border-2 border-ink bg-hl-yellow/75 text-ink"
                      : "border-2 border-transparent text-ink-soft hover:bg-paper-tint hover:text-ink"
                  }`}
                >
                  <span className="shrink-0 opacity-85">{item.icon}</span>
                  <span>{item.label}</span>
                </a>
              );
            })}
          </nav>

          {/* Tilted Language Highlighter Tags (jump straight to course syllabus) */}
          <div className="mt-7 space-y-2.5">
            <div className="font-mono text-[11px] font-bold tracking-wide text-ink-soft">
              {t.nav.builtInLabel}
            </div>
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                onClick={() => jumpToCourse("go")}
                className="-rotate-1 rounded-[8px] border-2 border-ink bg-hl-teal/25 px-2.5 py-1 font-mono text-xs font-bold text-ink transition-transform hover:rotate-0"
              >
                Go · 10
              </button>
              <button
                type="button"
                onClick={() => jumpToCourse("ts")}
                className="rotate-1 rounded-[8px] border-2 border-ink bg-hl-yellow px-2.5 py-1 font-mono text-xs font-bold text-ink transition-transform hover:rotate-0"
              >
                TS · 8
              </button>
              <button
                type="button"
                onClick={() => jumpToCourse("python")}
                className="rotate-1 rounded-[8px] border-2 border-ink bg-hl-teal/20 px-2.5 py-1 font-mono text-xs font-bold text-ink transition-transform hover:rotate-0"
              >
                Python · 6
              </button>
              <button
                type="button"
                onClick={() => jumpToCourse("java")}
                className="-rotate-1 rounded-[8px] border-2 border-ink bg-hl-pink/40 px-2.5 py-1 font-mono text-xs font-bold text-ink transition-transform hover:rotate-0"
              >
                Java 21 · 7
              </button>
            </div>
          </div>
        </div>

        {/* Bottom Zine Note */}
        <div className="rounded-[12px] border-2 border-hl-teal-deep bg-paper-bright p-3.5 text-xs leading-relaxed">
          <div className="font-mono font-bold text-hl-teal-deep">
            {t.nav.sidebarNoteTitle}
          </div>
          <p className="mt-1.5 text-ink-soft">{t.nav.sidebarNoteBody}</p>
        </div>
      </aside>

      {/* Main Notebook Canvas */}
      <main id="main-content" className="min-w-0">
        <div id="top" className="mx-auto max-w-[1120px] px-5 sm:px-8 lg:px-12">
          {/* Desktop Top Utility Strip */}
          <div className="hidden items-center justify-end gap-3 pt-6 lg:flex">
            <button
              type="button"
              onClick={() => setLocale(locale === "zh-CN" ? "en-US" : "zh-CN")}
              className="zine-btn-secondary px-3.5 py-2 font-mono text-xs"
            >
              <GlobeIcon size={14} />
              <span>{locale === "zh-CN" ? "English" : "简体中文"}</span>
            </button>
            <a
              href={REPO_URL}
              target="_blank"
              rel="noreferrer"
              className="zine-btn-secondary px-4 py-2 text-sm"
            >
              <GitHubIcon size={15} />
              <span>{t.nav.github}</span>
            </a>
            <a
              href={heroInstaller.downloadUrl}
              className="zine-btn-primary px-4 py-2 text-sm"
            >
              <DownloadIcon size={15} />
              <span>{t.nav.downloadShort}</span>
            </a>
          </div>

          {/* SECTION 1 — FIRST VIEWPORT HERO */}
          <section className="pb-10 pt-7 lg:pb-12 lg:pt-7">
            <div className="grid grid-cols-1 items-center gap-9 xl:grid-cols-12 xl:gap-8">
              {/* Left Thesis & Tick-Accented Primary Action */}
              <div className="xl:col-span-6">
                <h1 className="font-marker text-[2.05rem] leading-[1.2] tracking-[-0.02em] text-ink sm:text-[2.6rem] xl:text-[2.75rem]">
                  <span>{t.hero.titleLine1}</span>
                  <br />
                  <span className="swipe-pink whitespace-nowrap">{t.hero.titleHighlight}</span>
                  <span>{t.hero.titleLine2Rest}</span>
                </h1>
                <MarkerUnderline className="mt-3 h-3 w-64 max-w-full" />

                <p className="mt-5 max-w-[54ch] text-base leading-[1.7] text-ink-soft sm:text-[17px]">
                  {t.hero.subtitle}
                </p>

                <div className="mt-7 flex flex-wrap items-center gap-4">
                  <div className="inline-flex items-center gap-1.5">
                    <a
                      href={heroInstaller.downloadUrl}
                      data-primary-cta="true"
                      className="zine-btn-primary px-5 py-3.5 text-sm sm:px-6 sm:text-base"
                    >
                      <DownloadIcon size={18} />
                      <span>
                        {t.hero.downloadFor[detectedPlatform]} ({heroInstaller.sizeLabel})
                      </span>
                    </a>
                    <CtaEmphasisTicks className="hidden shrink-0 sm:block" />
                  </div>
                  <a
                    href="#try-it"
                    className="inline-flex items-center gap-1.5 border-b-[3px] border-hl-teal pb-1 text-[15px] font-bold text-ink transition-colors hover:text-hl-teal-deep"
                  >
                    <span>{t.hero.ctaSecondary}</span>
                    <ArrowDownIcon size={15} />
                  </a>
                </div>

                <p className="mt-4 font-mono text-xs text-ink-soft tabular">
                  {t.hero.metaLine}
                </p>
              </div>

              {/* Right Mechanism Plate: Tilted Static README slip -> In-flow SVG Arrow -> Interactive Go Card */}
              <div
                data-thesis-plate="true"
                className="flex flex-col xl:col-span-6"
              >
                {/* Tilted Static README Paper Slip */}
                <button
                  type="button"
                  onClick={() => {
                    setWorkbenchMode("static");
                    document.getElementById("try-it")?.scrollIntoView({ behavior: "smooth" });
                  }}
                  className="zine-plate w-full max-w-[310px] -rotate-1 p-3.5 text-left transition-transform hover:rotate-0 sm:p-4"
                >
                  <span className="inline-block rounded bg-paper-tint px-2 py-0.5 font-mono text-[11px] font-bold text-ink">
                    {t.hero.staticCardTag}
                  </span>
                  <div className="mt-2 font-mono text-xs leading-relaxed text-ink">
                    <div>{t.hero.staticCardLine1}</div>
                    <div className="text-ink-soft">{t.hero.staticCardLine2}</div>
                  </div>
                </button>

                {/* In-flow Animated Curved Teal Connector Arrow (never overlaps card borders; visible on mobile + desktop) */}
                <div className="pl-12 sm:pl-24">
                  <HeroFlowConnector key={`hero-conn-${heroMotionKey}`} />
                </div>

                {/* Live Interactive Workbench Preview Card (halftone focal mat + clickable 2/3 <-> 3/3 pass toggle) */}
                <div className="zine-plate-focal sm:ml-6">
                  <div className="zine-plate-focal-inner">
                    <div className="flex items-center justify-between gap-2 border-b-2 border-ink bg-paper-tint px-4 py-2.5 font-mono text-xs font-bold">
                      <span className="truncate text-ink">{t.hero.liveCardHeader}</span>
                      <button
                        type="button"
                        onClick={handleHeroPassToggle}
                        aria-pressed={heroDemoPassed}
                        className={`inline-flex shrink-0 items-center gap-1.5 rounded-full border-2 border-ink px-2.5 py-0.5 text-[11px] font-bold transition-colors ${
                          heroDemoPassed
                            ? "bg-hl-teal-deep text-white hover:bg-hl-teal-deep/90"
                            : "bg-hl-pink/40 text-ink hover:bg-hl-pink/60"
                        }`}
                      >
                        {heroDemoPassed ? <CheckIcon size={11} /> : <CrossIcon size={11} />}
                        <span>{heroDemoPassed ? t.hero.liveTogglePass : t.hero.liveToggleFail}</span>
                      </button>
                    </div>
                    <pre className="overflow-x-auto bg-paper-bright px-4 py-3.5 font-mono text-[12px] leading-[1.7] text-ink">
                      <code>
                        {"func routes() *http.ServeMux {\n"}
                        {"  mux := http.NewServeMux()\n"}
                        {"  mux.HandleFunc("}
                        <span
                          key={`hl-${heroDemoPassed}-${heroMotionKey}`}
                          className={`zine-swipe-reveal font-bold ${
                            heroDemoPassed ? "swipe-yellow" : "rounded bg-hl-pink/35 px-1.5 py-0.5 text-ink"
                          }`}
                        >
                          {heroDemoPassed ? '"GET /healthz"' : '"/healthz"'}
                        </span>
                        {", healthz)\n"}
                        {"  return mux\n"}
                        {"}"}
                      </code>
                    </pre>
                  </div>
                </div>

                {/* Curved Pink SVG Pointer Arrow + Tilted Margin Callout */}
                <div className="mt-2.5 flex items-start justify-end gap-1.5 pr-1">
                  <PinkPointerArrow key={`pink-ptr-${heroMotionKey}`} className="-mt-1 shrink-0" />
                  <span className="-rotate-1 pt-2 font-marker text-sm text-hl-pink-deep">
                    {heroDemoPassed ? t.hero.marginCalloutPass : t.hero.marginCalloutFail}
                  </span>
                </div>
              </div>
            </div>

            {/* Lower Fold Sequential 3-Step Strip (WCAG AA-safe #0B6E63 step counters) */}
            <div className="mt-9 border-t-2 border-ink pt-6">
              <div className="grid grid-cols-1 items-stretch gap-4 md:grid-cols-[1fr_auto_1fr_auto_1fr] md:gap-2">
                {t.hero.steps.map((step, index) => (
                  <div key={step.n} className="contents">
                    <div className="zine-plate p-4">
                      <div className="flex items-center gap-2.5">
                        <span className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full border-2 border-ink bg-hl-teal-deep font-mono text-xs font-extrabold text-white">
                          {step.n}
                        </span>
                        <h2 className="font-marker text-base text-ink">
                          {step.title}
                        </h2>
                      </div>
                      <p className="mt-2 text-xs leading-relaxed text-ink-soft sm:text-[13px]">
                        {step.desc}
                      </p>
                    </div>
                    {index < 2 && (
                      <div className="hidden items-center justify-center md:flex">
                        <SmallCurvedArrow />
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </section>

          {/* SECTION 2 — SIGNATURE INTERACTIVE WORKBENCH */}
          <InteractiveWorkbench
            locale={locale}
            mode={workbenchMode}
            onModeChange={setWorkbenchMode}
            passed={workbenchPassed}
            onPassChange={(next) => {
              setWorkbenchPassed(next);
              if (next) setHeroDemoPassed(true);
            }}
          />

          {/* SECTION 3 — REAL DESKTOP CAPTURE & OPEN ANY MARKDOWN */}
          <section id="markdown" className="scroll-mt-20 border-t-2 border-ink py-14 lg:py-20">
            <div className="max-w-2xl">
              <h2 className="font-marker text-3xl leading-tight tracking-[-0.015em] text-ink sm:text-4xl">
                {t.markdownSection.heading}
              </h2>
              <p className="mt-3 text-base leading-relaxed text-ink-soft">
                {t.markdownSection.lead}
              </p>
            </div>

            <div className="mt-8 grid grid-cols-1 items-start gap-8 lg:grid-cols-12">
              {/* Left: Real 2x Retina Desktop Screenshots framed as focal zine plate */}
              <div className="lg:col-span-7">
                <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
                  <div className="inline-flex rounded-[10px] border-2 border-ink bg-paper-bright p-1 text-xs font-bold">
                    <button
                      type="button"
                      onClick={() => setActiveScreenshot("workbench")}
                      className={`rounded-[7px] px-3 py-1.5 transition-colors ${
                        activeScreenshot === "workbench"
                          ? "bg-hl-yellow text-ink"
                          : "text-ink-soft hover:text-ink"
                      }`}
                    >
                      {t.markdownSection.tabWorkbench}
                    </button>
                    <button
                      type="button"
                      onClick={() => setActiveScreenshot("welcome")}
                      className={`rounded-[7px] px-3 py-1.5 transition-colors ${
                        activeScreenshot === "welcome"
                          ? "bg-hl-teal/25 text-ink"
                          : "text-ink-soft hover:text-ink"
                      }`}
                    >
                      {t.markdownSection.tabWelcome}
                    </button>
                  </div>
                  <span className="font-marker text-xs text-hl-teal-deep">
                    {t.markdownSection.calloutNote}
                  </span>
                </div>

                <figure className="zine-plate-focal">
                  <div className="zine-plate-focal-inner">
                    <img
                      src={
                        activeScreenshot === "workbench"
                          ? "./screenshots/workbench-live.png"
                          : "./screenshots/welcome-home.png"
                      }
                      alt={
                        activeScreenshot === "workbench"
                          ? t.markdownSection.captionWorkbench
                          : t.markdownSection.captionWelcome
                      }
                      width={1440}
                      height={900}
                      className="block h-auto w-full"
                      loading="lazy"
                    />
                    <figcaption className="border-t-2 border-ink bg-paper-tint px-4 py-2.5 font-mono text-xs text-ink-soft">
                      {activeScreenshot === "workbench"
                        ? t.markdownSection.captionWorkbench
                        : t.markdownSection.captionWelcome}
                    </figcaption>
                  </div>
                </figure>
              </div>

              {/* Right: 3 Clean Ink-Ruled Engineering Principles (icons, no arbitrary 01-03 numbering) */}
              <div className="divide-y-2 divide-ink border-y-2 border-ink lg:col-span-5">
                {t.markdownSection.pillars.map((pillar) => (
                  <div key={pillar.title} className="py-5">
                    <div className="flex items-start gap-3">
                      <span className="mt-1">{renderPillarIcon(pillar.icon)}</span>
                      <h3 className="font-marker text-lg text-ink">
                        {pillar.title}
                      </h3>
                    </div>
                    <p className="mt-2 pl-7 text-sm leading-relaxed text-ink-soft">
                      {pillar.body}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          </section>

          {/* SECTION 4 — BUILT-IN 4 COURSES & MANAGED RUNTIMES */}
          <CoursesExplorer
            locale={locale}
            activeCourse={activeCourse}
            onSelectCourse={setActiveCourse}
          />

          {/* SECTION 5 — LOCAL ACP AI TUTOR */}
          <TutorPreview locale={locale} />

          {/* SECTION 6 — TRANSPARENT v0.1.0 DOWNLOAD + AUTHOR CALLOUT */}
          <DownloadSection
            locale={locale}
            detectedPlatform={detectedPlatform}
            selectedPlatform={selectedPlatform}
            onSelectPlatform={setSelectedPlatform}
          />

          {/* FOOTER */}
          <footer className="border-t-2 border-ink py-8 text-sm">
            <div className="flex flex-col gap-5 sm:flex-row sm:items-center sm:justify-between">
              <div className="flex items-center gap-3">
                <span className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-full border-2 border-ink bg-hl-yellow">
                  <BrandMarkIcon size={15} />
                </span>
                <div>
                  <div className="font-marker text-base text-ink">CodeLab Studio</div>
                  <div className="text-xs text-ink-soft">{t.footer.tagline}</div>
                </div>
              </div>

              <div className="flex flex-wrap items-center gap-x-5 gap-y-2 text-xs font-semibold text-ink">
                <a href={REPO_URL} target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 hover:underline">
                  {t.footer.repo}
                  <ExternalIcon size={12} />
                </a>
                <a href={RELEASE_TAG_URL} target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 hover:underline">
                  {t.footer.releases}
                  <ExternalIcon size={12} />
                </a>
                <a
                  href={`${REPO_URL}/blob/${RELEASE_VERSION}/README.zh-CN.md`}
                  target="_blank"
                  rel="noreferrer"
                  className="hover:underline"
                >
                  {t.footer.docsZh}
                </a>
                <a
                  href={`${REPO_URL}/blob/${RELEASE_VERSION}/README.md`}
                  target="_blank"
                  rel="noreferrer"
                  className="hover:underline"
                >
                  {t.footer.docsEn}
                </a>
                <button
                  type="button"
                  onClick={() => setLocale(locale === "zh-CN" ? "en-US" : "zh-CN")}
                  className="rounded-md border border-ink bg-paper-bright px-2.5 py-1 font-mono text-[11px] font-bold"
                >
                  {locale === "zh-CN" ? "Switch to EN" : "切换简体中文"}
                </button>
              </div>
            </div>
          </footer>
        </div>
      </main>
    </div>
  );
}
