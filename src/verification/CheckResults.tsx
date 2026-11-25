import type { CheckResult, CourseCheckSpec, Locale } from "../core/contracts";
import { formatDomainError, type Messages } from "../i18n";

export function CheckResults({checks,results,messages,locale}: { checks: CourseCheckSpec[]; results: CheckResult[]; messages: Messages; locale: Locale }) {
  const t = messages.verification;
  return <div className="check-results" aria-live="polite">
    {results.length === 0 && <p className="verification-empty">{t.empty}</p>}
    {results.map(result => <article className="check-result" key={result.checkId}>
      <div className="v-row">
        <span className={`badge badge--${result.status === "passed" ? "success" : result.status === "running" ? "neutral" : "danger"}`}>{t.status[result.status]}</span>
        <span className="v-text">{checks.find(c=>c.id===result.checkId)?.title ?? result.checkId}</span>
        <span className="mono">{result.durationMs} ms</span>
      </div>
      {result.error && <p className="verification-error" role="alert">{formatDomainError(result.error,locale)}</p>}
      {result.diffs.map((diff,index)=><div className="check-diff" key={index}>
        <span className="check-diff-label mono">{diff.label}</span>
        <div><span>{t.expected}</span><pre className="mono">{diff.expected}</pre></div>
        <div><span>{t.actual}</span><pre className="mono">{diff.actual}</pre></div>
      </div>)}
      {result.testCases.map(test=><details className="check-test" key={test.name}>
        <summary><span>{t.status[test.status]}</span> <span className="mono">{test.name} · {test.durationMs} ms</span></summary>
        <pre className="mono">{test.output}</pre>
      </details>)}
      {result.rawOutput && <details className="check-raw"><summary>{t.raw}</summary><pre className="mono">{result.rawOutput}</pre></details>}
    </article>)}
  </div>;
}
