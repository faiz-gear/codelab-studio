import { SparklesIcon } from "../components/Icons";
import type { CheckResult, CourseCheckSpec, Locale } from "../core/contracts";
import { formatDomainError, interpolate, type Messages } from "../i18n";

export function CheckResults({
  checks,
  results,
  messages,
  locale,
  onAskCheck,
  onAskAll,
}: {
  checks: CourseCheckSpec[];
  results: CheckResult[];
  messages: Messages;
  locale: Locale;
  onAskCheck?: (check: CourseCheckSpec | undefined, result: CheckResult) => void;
  onAskAll?: () => void;
}) {
  const t = messages.verification;
  const tc = messages.tutor.context;
  return (
    <div className="check-results" aria-live="polite">
      {results.length === 0 && <p className="verification-empty">{t.empty}</p>}
      {results.length > 0 && onAskAll && (
        <div className="check-results-head">
          <button
            type="button"
            className="btn btn--ai-soft btn--xs btn--pill"
            data-testid="verification-panel-ask-ai"
            onClick={onAskAll}
          >
            <SparklesIcon size={12} />
            <span>{tc.askAiVerificationPanel}</span>
          </button>
        </div>
      )}
      {results.map((result) => {
        const spec = checks.find((c) => c.id === result.checkId);
        const title = spec?.title ?? result.checkId;
        const failed = result.status === "failed" || result.status === "timedOut";
        return (
          <article className="check-result" key={result.checkId}>
            <div className="v-row">
              <span
                className={`badge badge--${result.status === "passed" ? "success" : result.status === "running" ? "neutral" : "danger"}`}
              >
                {t.status[result.status]}
              </span>
              <span className="v-text">{title}</span>
              <span className="mono">{result.durationMs} ms</span>
              {failed && onAskCheck && (
                <button
                  type="button"
                  className="btn btn--ai-soft btn--xs btn--pill"
                  aria-label={interpolate(tc.askAiCheckFailure, { title })}
                  data-testid={`check-result-ask-ai-${result.checkId}`}
                  onClick={() => onAskCheck(spec, result)}
                >
                  <SparklesIcon size={11} />
                  <span>{tc.askAiAction}</span>
                </button>
              )}
            </div>
            {result.error && (
              <p className="verification-error" role="alert">
                {formatDomainError(result.error, locale)}
              </p>
            )}
            {result.diffs.map((diff, index) => (
              <div className="check-diff" key={index}>
                <span className="check-diff-label mono">{diff.label}</span>
                <div>
                  <span>{t.expected}</span>
                  <pre className="mono">{diff.expected}</pre>
                </div>
                <div>
                  <span>{t.actual}</span>
                  <pre className="mono">{diff.actual}</pre>
                </div>
              </div>
            ))}
            {result.testCases.map((test) => (
              <details className="check-test" key={test.name}>
                <summary>
                  <span>{t.status[test.status]}</span>{" "}
                  <span className="mono">
                    {test.name} · {test.durationMs} ms
                  </span>
                </summary>
                <pre className="mono">{test.output}</pre>
              </details>
            ))}
            {result.rawOutput && (
              <details className="check-raw">
                <summary>{t.raw}</summary>
                <pre className="mono">{result.rawOutput}</pre>
              </details>
            )}
          </article>
        );
      })}
    </div>
  );
}
