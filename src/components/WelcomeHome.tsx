import { useState } from "react";
import {
  COURSE_LANGUAGES,
  type CourseLanguage,
  type CourseSummary,
  type Locale,
  type RecentWorkspace,
} from "../core/contracts";
import {
  COURSE_PLACEHOLDER_META,
  formatRelativeTime,
  interpolate,
  type Messages,
} from "../i18n";
import { FolderDownIcon, FolderIcon, PlayIcon } from "./Icons";

export type LanguageFilter = "all" | CourseLanguage;

export interface WelcomeHomeProps {
  messages: Messages;
  locale: Locale;
  builtinCourses: CourseSummary[];
  recents: RecentWorkspace[];
  onOpenBuiltinCourse: (courseId: string) => void;
  onOpenRecentWorkspace: (workspacePath: string) => void;
  onOpenImportModal: () => void;
  onOpenAuthorModal: () => void;
  onPlaceholderAction: (target: string) => void;
}

export function WelcomeHome({
  messages,
  locale,
  builtinCourses,
  recents,
  onOpenBuiltinCourse,
  onOpenRecentWorkspace,
  onOpenImportModal,
  onOpenAuthorModal,
  onPlaceholderAction,
}: WelcomeHomeProps) {
  const [languageFilter, setLanguageFilter] = useState<LanguageFilter>("all");
  const latestRecent = recents[0] ?? null;

  const progressByCourse = new Map<string, RecentWorkspace>();
  for (const recent of recents) {
    if (!progressByCourse.has(recent.courseId)) {
      progressByCourse.set(recent.courseId, recent);
    }
  }

  const catalogCourses =
    builtinCourses.length > 0
      ? builtinCourses.map((course) => ({
          id: course.courseId,
          language: course.language,
          totalExercises: course.totalExercises,
          title:
            messages.welcome.courseTexts[course.courseId]?.title ?? course.title,
          description:
            messages.welcome.courseTexts[course.courseId]?.description ??
            course.description,
          builtin: course,
        }))
      : COURSE_PLACEHOLDER_META.map((course) => ({
          id: course.id,
          language: course.language,
          totalExercises: course.totalExercises,
          title: messages.welcome.courseTexts[course.id]?.title ?? course.id,
          description:
            messages.welcome.courseTexts[course.id]?.description ?? "",
          builtin: undefined,
        }));

  const visibleCourses = catalogCourses.filter(
    (course) => languageFilter === "all" || course.language === languageFilter
  );

  const primaryCtaLabel = latestRecent
    ? interpolate(messages.welcome.primaryCtaContinue, {
        title: latestRecent.courseTitle,
      })
    : messages.welcome.primaryCtaStart;

  const handlePrimaryCta = () => {
    if (latestRecent) {
      onOpenRecentWorkspace(latestRecent.workspacePath);
      return;
    }
    const firstBuiltin = builtinCourses[0];
    if (firstBuiltin) {
      onOpenBuiltinCourse(firstBuiltin.courseId);
      return;
    }
    onPlaceholderAction(primaryCtaLabel);
  };

  return (
    <section
      className="welcome-scroll"
      aria-label={messages.welcome.ariaLabel}
    >
      <div className="welcome-inner">
        <section className="hero-panel">
          <h1>{messages.welcome.heroTitle}</h1>
          <p className="hero-sub">{messages.welcome.heroSubtitle}</p>

          <div className="hero-actions">
            <button
              type="button"
              className="btn btn--primary btn--lg"
              onClick={handlePrimaryCta}
            >
              <PlayIcon />
              <span>{primaryCtaLabel}</span>
            </button>
            <button
              type="button"
              className="btn btn--secondary btn--lg"
              onClick={onOpenImportModal}
            >
              <FolderDownIcon />
              <span>{messages.welcome.secondaryCtaImport}</span>
            </button>
          </div>

          <div className="hero-status">
            <span className="dot dot--ok" aria-hidden="true" />
            {latestRecent ? (
              <span>
                {interpolate(messages.welcome.resumeSummaryPrefix, {
                  title: latestRecent.courseTitle,
                })}{" "}
                <span className="mono">
                  ({latestRecent.completedExercises} / {latestRecent.totalExercises})
                </span>
              </span>
            ) : (
              <span>{messages.welcome.localFirstSummary}</span>
            )}
          </div>

          <div className="hero-foot">
            <span>{messages.welcome.footerDesktop}</span>
            <span aria-hidden="true">·</span>
            <span>{messages.welcome.footerLanguages}</span>
            <span aria-hidden="true">·</span>
            <span>{messages.welcome.footerTutor}</span>
          </div>
        </section>

        <section className="home-section">
          <div className="section-head">
            <h2>{messages.welcome.coursesHeading}</h2>
            <div
              className="seg seg--pill"
              role="group"
              aria-label={messages.welcome.filterGroupLabel}
            >
              <button
                type="button"
                aria-pressed={languageFilter === "all"}
                onClick={() => setLanguageFilter("all")}
              >
                {messages.welcome.filterAll}
              </button>
              {COURSE_LANGUAGES.map((lang) => (
                <button
                  key={lang}
                  type="button"
                  aria-pressed={languageFilter === lang}
                  onClick={() => setLanguageFilter(lang)}
                >
                  {lang}
                </button>
              ))}
            </div>
          </div>

          <div className="course-grid" data-testid="course-grid">
            {visibleCourses.map((course) => {
              const builtin = course.builtin;
              const matchedRecent = progressByCourse.get(course.id);
              const completed = matchedRecent?.completedExercises ?? 0;
              const total =
                matchedRecent?.totalExercises ??
                builtin?.totalExercises ??
                course.totalExercises;
              const pct =
                total > 0
                  ? Math.min(100, Math.round((completed / total) * 100))
                  : 0;

              return (
                <button
                  key={course.id}
                  type="button"
                  className="course-card"
                  onClick={() => {
                    if (builtin) {
                      onOpenBuiltinCourse(builtin.courseId);
                    } else {
                      onPlaceholderAction(course.title);
                    }
                  }}
                >
                  <div className="course-card-top">
                    {course.language ? (
                      <span className="lang-tag" data-lang={course.language}>
                        {course.language}
                      </span>
                    ) : null}
                    <span className="mono muted" style={{ fontSize: "12px" }}>
                      {completed} / {total}
                    </span>
                  </div>
                  <h3>{course.title}</h3>
                  <p>{course.description}</p>
                  <div className="course-card-foot">
                    <div className="prog-track" aria-hidden="true">
                      <div className="prog-fill" style={{ width: `${pct}%` }} />
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        </section>

        <section className="home-section">
          <div className="section-head">
            <h2>{messages.welcome.recentHeading}</h2>
            <button
              type="button"
              className="btn btn--ghost btn--xs"
              onClick={onOpenAuthorModal}
            >
              {messages.welcome.newCoursePack}
            </button>
          </div>

          <div className="recent-list" data-testid="recent-workspace-list">
            {recents.length === 0 ? (
              <div className="recent-empty">
                <p className="recent-empty-title">
                  {messages.welcome.recentEmptyTitle}
                </p>
                <p className="recent-empty-hint">
                  {messages.welcome.recentEmptyHint}
                </p>
              </div>
            ) : (
              recents.map((item) => (
                <button
                  key={item.workspacePath}
                  type="button"
                  className="recent-row"
                  onClick={() => onOpenRecentWorkspace(item.workspacePath)}
                >
                  <span className="muted" aria-hidden="true">
                    <FolderIcon />
                  </span>
                  <span className="recent-title">{item.courseTitle}</span>
                  <span className="mono recent-path" title={item.workspacePath}>
                    {item.workspacePath}
                  </span>
                  <span className="mono recent-meta">
                    {item.completedExercises}/{item.totalExercises}
                  </span>
                  <span className="recent-meta">
                    {formatRelativeTime(item.lastOpenedAt, locale)}
                  </span>
                </button>
              ))
            )}
          </div>
        </section>
      </div>
    </section>
  );
}
