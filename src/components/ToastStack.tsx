export type ToastTone = "ok" | "warn" | "err" | "info";

export interface ToastMessage {
  id: number;
  tone: ToastTone;
  text: string;
}

export interface ToastStackProps {
  toasts: ToastMessage[];
}

export function ToastStack({ toasts }: ToastStackProps) {
  return (
    <div className="toasts" role="status" aria-live="polite" aria-atomic="true">
      {toasts.map((toast) => (
        <div key={toast.id} className="toast">
          <span
            className={`dot ${
              toast.tone === "ok"
                ? "dot--ok"
                : toast.tone === "warn"
                  ? "dot--warn"
                  : toast.tone === "err"
                    ? "dot--err"
                    : "dot--muted"
            }`}
            aria-hidden="true"
          />
          <span>{toast.text}</span>
        </div>
      ))}
    </div>
  );
}
