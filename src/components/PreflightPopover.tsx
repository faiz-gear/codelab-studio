import { useEffect, useId, useRef, useState } from "react";
import { interpolate } from "../i18n";
import { RuntimePanel, type RuntimePanelProps } from "./RuntimePanel";

export function PreflightPopover({
  onOpenSettings,
  ...panel
}: RuntimePanelProps & { onOpenSettings: (trigger: HTMLElement) => void }) {
  const { report, messages } = panel;
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const id = useId();

  useEffect(() => {
    if (!open) return;
    heading.current?.focus();
    function dismissOutside(event: Event) {
      if (
        event.target instanceof Node &&
        !anchor.current?.contains(event.target)
      ) {
        setOpen(false);
      }
    }
    document.addEventListener("pointerdown", dismissOutside);
    document.addEventListener("focusin", dismissOutside);
    return () => {
      document.removeEventListener("pointerdown", dismissOutside);
      document.removeEventListener("focusin", dismissOutside);
    };
  }, [open]);

  const total = report?.languages.length ?? 0;
  const ready =
    report?.languages.filter((row) => row.status === "ok").length ?? 0;
  const installing = report?.installingCount ?? 0;
  const status = installing
    ? interpolate(messages.runtime.installingCount, { count: installing })
    : report?.checked && total > 0
      ? interpolate(messages.runtime.readyCount, { ready, total })
      : "";
  const tone = installing
    ? "warn"
    : !status
      ? "muted"
      : ready === total
        ? "ok"
        : "warn";

  return (
    <div
      ref={anchor}
      className="preflight-anchor"
      onKeyDown={(event) => {
        if (open && event.key === "Escape") {
          event.stopPropagation();
          setOpen(false);
          trigger.current?.focus();
        }
      }}
    >
      <button
        ref={trigger}
        type="button"
        className="btn btn--ghost btn--sm btn--pill"
        title={messages.topbar.envStatusTooltip}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={open ? id : undefined}
        onClick={() => setOpen(!open)}
      >
        <span className={`dot dot--${tone}`} aria-hidden="true" />
        <span>{messages.topbar.envStatusLabel}</span>
        {status && <span className="mono preflight-count">{status}</span>}
      </button>
      {open && (
        <section
          id={id}
          className="preflight-popover"
          role="dialog"
          aria-labelledby={`${id}-title`}
        >
          <header className="preflight-head">
            <h2 id={`${id}-title`} ref={heading} tabIndex={-1}>
              {messages.topbar.envStatusLabel}
            </h2>
            <button
              type="button"
              className="btn btn--ghost btn--xs"
              onClick={() => {
                setOpen(false);
                if (trigger.current) onOpenSettings(trigger.current);
              }}
            >
              {messages.runtime.manage}
            </button>
          </header>
          <RuntimePanel {...panel} />
        </section>
      )}
    </div>
  );
}
