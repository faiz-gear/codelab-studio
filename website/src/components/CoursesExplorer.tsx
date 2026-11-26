import { BUILTIN_COURSES, COPY, type CourseId, type Locale } from "../data/content";
import { CheckIcon, CodeFileIcon, ShieldIcon, TerminalIcon } from "./Icons";

interface CoursesExplorerProps {
  locale: Locale;
  activeCourse: CourseId;
  onSelectCourse: (id: CourseId) => void;
}

const TAB_ACCENTS: Record<"teal" | "yellow" | "pink", string> = {
  teal: "bg-hl-teal/25 text-ink",
  yellow: "bg-hl-yellow text-ink",
  pink: "bg-hl-pink/40 text-ink",
};

export function CoursesExplorer({
  locale,
  activeCourse,
  onSelectCourse,
}: CoursesExplorerProps) {
  const t = COPY[locale].coursesSection;
  const current = BUILTIN_COURSES.find((c) => c.id === activeCourse) ?? BUILTIN_COURSES[0];

  return (
    <section id="courses" className="scroll-mt-20 border-t-2 border-ink py-14 lg:py-20">
      <div className="max-w-2xl">
        <h2 className="font-marker text-3xl leading-tight tracking-[-0.015em] text-ink sm:text-4xl">
          {t.heading}
        </h2>
        <p className="mt-3 text-base leading-relaxed text-ink-soft">
          {t.lead}
        </p>
      </div>

      {/* Notebook Folder Tabs (no meaningless 01-04 numbers; single-line 2x2 grid on 390px mobile) */}
      <div className="mt-8">
        <div
          role="tablist"
          aria-label={locale === "zh-CN" ? "内置四语言课程切换" : "Built-in course tabs"}
          className="grid grid-cols-2 gap-2 pb-2 sm:flex sm:flex-wrap sm:pb-0"
        >
          {BUILTIN_COURSES.map((course) => {
            const selected = course.id === activeCourse;
            return (
              <button
                key={course.id}
                type="button"
                role="tab"
                aria-selected={selected}
                onClick={() => onSelectCourse(course.id)}
                className={`inline-flex items-center justify-between gap-2 whitespace-nowrap rounded-[11px] border-2 border-ink px-2.5 py-2 text-xs font-bold transition-colors sm:justify-start sm:rounded-b-none sm:rounded-t-[12px] sm:border-b-0 sm:px-4 sm:py-2.5 sm:text-sm ${
                  selected
                    ? `${TAB_ACCENTS[course.accent]} sm:relative sm:z-10 sm:-mb-[2px] sm:pb-3`
                    : "bg-paper-tint text-ink-soft hover:bg-paper-bright hover:text-ink"
                }`}
              >
                <span>{course.badge}</span>
                <span className="whitespace-nowrap rounded-full bg-paper-bright px-2 py-0.5 font-mono text-[11px] font-semibold text-ink tabular">
                  {course.exerciseCount} {t.exercisesUnit}
                </span>
              </button>
            );
          })}
        </div>

        {/* Single-sheet Course Plate (zero nested bordered boxes inside) */}
        <div className="zine-plate overflow-hidden sm:rounded-tl-none">
          <div className="grid grid-cols-1 lg:grid-cols-12">
            {/* Left overview + borderless tonal code band */}
            <div className="flex flex-col justify-between border-b-2 border-ink bg-paper-bright lg:col-span-6 lg:border-b-0 lg:border-r-2">
              <div className="p-6 sm:p-7">
                <h3 className="font-marker text-2xl text-ink">
                  {current.title[locale]}
                </h3>
                <p className="mt-3 text-[15px] leading-relaxed text-ink-soft">
                  {current.summary[locale]}
                </p>

                <dl className="mt-6 grid grid-cols-1 gap-3 border-t-2 border-ink/15 pt-4 text-xs sm:grid-cols-3">
                  <div>
                    <dt className="font-medium text-ink-soft">{t.runtimeLabel}</dt>
                    <dd className="mt-1 font-mono font-bold text-ink">{current.runtime}</dd>
                  </div>
                  <div>
                    <dt className="font-medium text-ink-soft">{t.runnerLabel}</dt>
                    <dd className="mt-1 font-mono font-bold text-hl-teal-deep">
                      {current.runner[locale]}
                    </dd>
                  </div>
                  <div>
                    <dt className="font-medium text-ink-soft">{t.entryLabel}</dt>
                    <dd className="mt-1 truncate font-mono font-bold text-ink">{current.entryFile}</dd>
                  </div>
                </dl>
              </div>

              <div className="border-t-2 border-ink/20 bg-paper-tint px-6 py-4 sm:px-7">
                <div className="mb-2 flex items-center gap-2 font-mono text-[11px] font-bold text-ink-soft">
                  <CodeFileIcon size={14} className="shrink-0 text-hl-teal-deep" />
                  <span>{current.entryFile}</span>
                </div>
                <pre className="overflow-x-auto font-mono text-[12.5px] leading-relaxed text-ink">
                  {current.snippet}
                </pre>
              </div>
            </div>

            {/* Right syllabus list */}
            <div className="bg-paper p-6 sm:p-7 lg:col-span-6">
              <div className="mb-4 flex items-center justify-between">
                <h4 className="font-marker text-lg text-ink">
                  {t.syllabusHeading}
                </h4>
                <span className="font-mono text-xs font-bold text-ink-soft tabular">
                  {current.exerciseCount} {t.exercisesUnit}
                </span>
              </div>
              <ol className="divide-y divide-ink/15">
                {current.chapters[locale].map((chapter, i) => (
                  <li
                    key={chapter}
                    className="flex items-center gap-3 py-2.5 text-sm text-ink"
                  >
                    <span className="w-6 shrink-0 font-mono text-xs font-bold text-hl-teal-deep tabular">
                      {String(i + 1).padStart(2, "0")}
                    </span>
                    <span className="leading-snug">{chapter}</span>
                  </li>
                ))}
              </ol>
            </div>
          </div>
        </div>
      </div>

      {/* Open Notebook Managed Runtime Isolation Band (un-nested directly on paper ground) */}
      <div className="mt-8 border-t-2 border-ink/25 pt-6">
        <div className="flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between">
          <div className="max-w-2xl">
            <div className="flex items-center gap-2 font-bold text-ink">
              <ShieldIcon size={18} className="shrink-0 text-hl-teal-deep" />
              <h3 className="font-marker text-lg text-ink">{t.preflightTitle}</h3>
            </div>
            <p className="mt-2 text-sm leading-relaxed text-ink-soft">
              {t.preflightBody}
            </p>
          </div>
          <div className="flex flex-wrap gap-2 lg:max-w-sm lg:justify-end">
            {t.runtimePills.map((pill) => (
              <span
                key={pill}
                className="inline-flex items-center gap-1.5 rounded-full border-2 border-ink bg-paper-bright px-3 py-1 font-mono text-xs font-semibold text-ink"
              >
                <CheckIcon size={12} className="shrink-0 text-hl-teal-deep" />
                {pill}
              </span>
            ))}
          </div>
        </div>

        {/* Non-wrapping icon + footnote row (fixes orphaned icon on 390px mobile) */}
        <div className="mt-4 flex items-start gap-2 font-mono text-xs leading-relaxed text-ink-soft">
          <TerminalIcon size={14} className="mt-0.5 shrink-0 text-ink" />
          <span>
            {locale === "zh-CN"
              ? "可选语言服务按最近 Module Root 懒加载（gopls / typescript-language-server / pyright / jdtls），未安装时自动保持语法高亮。"
              : "Optional LSPs attach per nearest Module Root (gopls / typescript-language-server / pyright / jdtls) and degrade cleanly to syntax highlighting."}
          </span>
        </div>
      </div>
    </section>
  );
}
