import { useEffect, useRef } from "react";
import { type Locale, t } from "../../i18n";
import type { CommandBlock, ParsedChapter, ParsedCourse } from "../../lib/ipc/generated";
import { Markdown } from "./Markdown";
import "./course-preview.css";

export function CoursePreview({ course, locale, onClose }: {
  course: ParsedCourse;
  locale: Locale;
  onClose: () => void;
}) {
  const title = useRef<HTMLHeadingElement>(null);
  useEffect(() => title.current?.focus(), []);

  return (
    <main className="course-preview">
      <aside className="preview-sidebar">
        <button type="button" className="preview-close" onClick={onClose}>{t(locale, "preview.close")}</button>
        <nav aria-label={t(locale, "preview.chapters")}>
          <ol>
            {course.chapters.map((chapter) => (
              <li key={chapter.chapterId}><a href={`#manual-${chapter.chapterId}`}>{chapter.title}</a></li>
            ))}
          </ol>
        </nav>
      </aside>
      <article className="preview-manual" aria-labelledby="preview-title">
        <h1 id="preview-title" ref={title} tabIndex={-1}>{course.title}</h1>
        <Markdown nodes={course.introNodes} />
        {course.chapters.map((chapter) => (
          <section key={chapter.chapterId} aria-labelledby={`manual-${chapter.chapterId}`}>
            <h2 id={`manual-${chapter.chapterId}`}>{chapter.title}</h2>
            <Markdown nodes={chapter.nodes} />
            {chapter.kind === "exercise" && <ExerciseSteps chapter={chapter} locale={locale} />}
          </section>
        ))}
      </article>
    </main>
  );
}

function ExerciseSteps({ chapter, locale }: { chapter: ParsedChapter; locale: Locale }) {
  const flow = chapter.threeStep;
  const commandById = new Map(chapter.commandBlocks.map((command) => [command.id, command]));

  return (
    <details className="exercise-steps">
      <summary>{t(locale, "preview.steps")}</summary>
      <ol>
        <li>
          <h3>{t(locale, "preview.run")}</h3>
          {flow.step1PrimaryCommands.map((id) => {
            const command = commandById.get(id);
            return command ? <Command key={id} command={command} locale={locale} /> : null;
          })}
          {flow.secondaryCommands.length > 0 && (
            <details>
              <summary>{t(locale, "preview.secondary")}</summary>
              {flow.secondaryCommands.map((id) => {
                const command = commandById.get(id);
                return command ? <Command key={id} command={command} locale={locale} /> : null;
              })}
            </details>
          )}
        </li>
        <li>
          <h3>{t(locale, "preview.implement")}</h3>
          {chapter.goal && <p>{chapter.goal}</p>}
          {flow.step2ReferenceBlocks.map((block) => (
            <figure key={block.codeIndex}>
              {(block.targetPath || block.language) && <figcaption>{block.targetPath || block.language}</figcaption>}
              <pre><code>{block.code}</code></pre>
            </figure>
          ))}
        </li>
        <li>
          <h3>{t(locale, "preview.verify")}</h3>
          <ul>
            {flow.step3SelfChecks.map((item) => <li key={item.id}>{item.title}</li>)}
          </ul>
          <Markdown nodes={flow.reflectionNodes} />
        </li>
      </ol>
    </details>
  );
}

function Command({ command, locale }: { command: CommandBlock; locale: Locale }) {
  return (
    <figure className="preview-command">
      <figcaption>
        {command.label && <span>{command.label}</span>}
        <span>{t(locale, command.role.kind === "daemon" ? "preview.daemon" : "preview.oneshot")}</span>
        <code>{t(locale, "preview.cwd", { path: command.cwd ?? "." })}</code>
      </figcaption>
      <pre><code>{command.command}</code></pre>
    </figure>
  );
}
