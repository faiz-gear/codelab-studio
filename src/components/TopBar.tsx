import { useEffect, useRef, useState, type RefObject } from "react";
import {
  courseSourceKey,
  type CourseSummary,
  type LoadedCourse,
} from "../core/contracts";
import type { Messages } from "../i18n";
import { PreflightPopover } from "./PreflightPopover";
import type { RuntimePanelProps } from "./RuntimePanel";
import {
  BrandMarkIcon,
  ChevronDownIcon,
  FolderDownIcon,
  HomeIcon,
  PanelLeftCloseIcon,
  PanelLeftOpenIcon,
  SettingsIcon,
  SlidersIcon,
  SparklesIcon,
} from "./Icons";

export interface TopBarProps {
  messages: Messages;
  builtinCourses: CourseSummary[];
  importedCourses: CourseSummary[];
  activeCourse: LoadedCourse | null;
  completedExercises?: number;
  focusMode: boolean;
  tutorOpen?: boolean;
  tutorTriggerRef?: RefObject<HTMLButtonElement | null>;
  appearanceOpen: boolean;
  appearanceTriggerRef: RefObject<HTMLButtonElement | null>;
  onGoHome: () => void;
  onSelectCourse: (course: CourseSummary) => void;
  onOpenImportModal: () => void;
  onToggleFocusMode: () => void;
  onToggleTutor?: () => void;
  onOpenAppearance: () => void;
  runtimePanel: RuntimePanelProps;
  runtimeSettingsOpen: boolean;
  onOpenRuntimeSettings: (trigger: HTMLElement) => void;
}

export function TopBar({
  messages,
  builtinCourses,
  importedCourses,
  activeCourse,
  completedExercises = 0,
  focusMode,
  tutorOpen = false,
  tutorTriggerRef,
  appearanceOpen,
  appearanceTriggerRef,
  onGoHome,
  onSelectCourse,
  onOpenImportModal,
  onToggleFocusMode,
  onToggleTutor,
  onOpenAppearance,
  runtimePanel,
  runtimeSettingsOpen,
  onOpenRuntimeSettings,
}: TopBarProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!menuOpen) return;
    const handlePointerDown = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setMenuOpen(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [menuOpen]);

  const activeSourceKey = activeCourse && courseSourceKey(activeCourse.source);
  const builtinTitle = activeCourse?.isBuiltin
    ? messages.welcome.courseTexts[activeCourse.courseId]?.title
    : undefined;
  const activeTitle = builtinTitle ?? activeCourse?.title ?? messages.topbar.courseMenuEmpty;

  return (
    <header className="topbar">
      <button
        type="button"
        className="brand"
        title={messages.topbar.homeTooltip}
        onClick={onGoHome}
      >
        <span className="brand-mark">
          <BrandMarkIcon />
        </span>
        <span>{messages.brandTitle}</span>
      </button>

      <div ref={dropdownRef} className="course-dd">
        <button
          type="button"
          className="course-trigger"
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          aria-label={messages.topbar.courseMenuLabel}
          onClick={() => setMenuOpen((open) => !open)}
        >
          {activeCourse?.language ? (
            <span className="lang-tag" data-lang={activeCourse.language}>
              {activeCourse.language}
            </span>
          ) : null}
          <span className="course-trigger-title">{activeTitle}</span>
          <ChevronDownIcon size={13} />
        </button>

        {menuOpen ? (
          <div
            className="dd-menu"
            role="menu"
            aria-label={messages.topbar.courseMenuLabel}
          >
            {builtinCourses.length > 0 ? (
              <div className="dd-group">
                <div className="dd-group-label">
                  {messages.topbar.builtinGroupLabel}
                </div>
                {builtinCourses.map((course) => {
                  const displayTitle =
                    messages.welcome.courseTexts[course.courseId]?.title ??
                    course.title;
                  const isActive = activeSourceKey === courseSourceKey(course.source);
                  return (
                    <button
                      key={course.courseId}
                      type="button"
                      role="menuitem"
                      className={`dd-item${isActive ? " is-active" : ""}`}
                      onClick={() => {
                        setMenuOpen(false);
                        onSelectCourse(course);
                      }}
                    >
                      {course.language ? (
                        <span className="lang-tag" data-lang={course.language}>
                          {course.language}
                        </span>
                      ) : null}
                      <span className="dd-item-title">{displayTitle}</span>
                      <span className="mono muted dd-item-meta">
                        {course.totalExercises}
                      </span>
                    </button>
                  );
                })}
              </div>
            ) : null}

            {importedCourses.length > 0 ? (
              <div className="dd-group">
                <div className="dd-group-label">
                  {messages.topbar.importedGroupLabel}
                </div>
                {importedCourses.map((course) => {
                  const isActive = activeSourceKey === courseSourceKey(course.source);
                  return (
                    <button
                      key={courseSourceKey(course.source)}
                      type="button"
                      role="menuitem"
                      className={`dd-item${isActive ? " is-active" : ""}`}
                      onClick={() => {
                        setMenuOpen(false);
                        onSelectCourse(course);
                      }}
                    >
                      {course.language ? (
                        <span className="lang-tag" data-lang={course.language}>
                          {course.language}
                        </span>
                      ) : null}
                      <span className="dd-item-title">{course.title}</span>
                      <span className="mono muted dd-item-meta">
                        {course.totalChapters}
                      </span>
                    </button>
                  );
                })}
              </div>
            ) : null}

            <div className="dd-sep" role="separator" />

            <button
              type="button"
              role="menuitem"
              className="dd-item"
              onClick={() => {
                setMenuOpen(false);
                onOpenImportModal();
              }}
            >
              <FolderDownIcon size={14} />
              <span className="dd-item-title">
                {messages.topbar.importLocalAction}
              </span>
            </button>

            {activeCourse ? (
              <button
                type="button"
                role="menuitem"
                className="dd-item"
                onClick={() => {
                  setMenuOpen(false);
                  onGoHome();
                }}
              >
                <HomeIcon size={14} />
                <span className="dd-item-title">
                  {messages.topbar.backToHomeAction}
                </span>
              </button>
            ) : null}
          </div>
        ) : null}
      </div>

      {activeCourse ? (
        <span
          className="badge badge--neutral mono topbar-progress"
          title={messages.topbar.progressLabel}
          data-testid="topbar-exercise-progress"
        >
          {completedExercises} / {activeCourse.totalExercises}
        </span>
      ) : null}

      <div className="top-right">
        {activeCourse ? (
          <button
            type="button"
            className="btn btn--ghost btn--sm btn--pill"
            aria-label={messages.topbar.focusModeLabel}
            aria-pressed={focusMode}
            title={messages.topbar.focusModeTooltip}
            onClick={onToggleFocusMode}
          >
            {focusMode ? (
              <PanelLeftOpenIcon size={14} />
            ) : (
              <PanelLeftCloseIcon size={14} />
            )}
            <span>{messages.topbar.focusModeLabel}</span>
          </button>
        ) : null}

        {activeCourse && onToggleTutor ? (
          <button
            ref={tutorTriggerRef}
            type="button"
            className={`btn btn--sm btn--pill ${tutorOpen ? "btn--ai-soft" : "btn--ghost"}`}
            aria-expanded={tutorOpen}
            aria-controls="tutor-drawer"
            title={messages.topbar.tutorTooltip}
            onClick={onToggleTutor}
          >
            <SparklesIcon size={14} />
            <span>{messages.topbar.tutorLabel}</span>
          </button>
        ) : null}

        <PreflightPopover {...runtimePanel} onOpenSettings={onOpenRuntimeSettings} />

        <button
          ref={appearanceTriggerRef}
          type="button"
          className="btn btn--ghost btn--sm btn--pill"
          title={messages.topbar.appearanceTooltip}
          aria-expanded={appearanceOpen}
          aria-haspopup="dialog"
          onClick={onOpenAppearance}
        >
          <SlidersIcon />
          <span>{messages.topbar.appearance}</span>
        </button>

        <button
          type="button"
          className="btn btn--ghost btn--icon btn--sm btn--pill"
          aria-label={messages.topbar.settingsLabel}
          title={messages.topbar.settingsTooltip}
          aria-haspopup="dialog"
          aria-expanded={runtimeSettingsOpen}
          onClick={(event) => onOpenRuntimeSettings(event.currentTarget)}
        >
          <SettingsIcon />
        </button>
      </div>
    </header>
  );
}
