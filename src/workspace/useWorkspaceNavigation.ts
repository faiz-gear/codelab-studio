import { useRef, useState } from "react";
import type { WorkspaceEditorSession } from "./editorSession";

type Transition = () => Promise<void>;

export function useWorkspaceNavigation(
  editor: Pick<WorkspaceEditorSession, "hasDrafts" | "save"> | null,
  onError: (error: unknown) => void,
) {
  const [pending, setPending] = useState<{ run: Transition } | null>(null);
  const [switching, setSwitching] = useState(false);
  const running = useRef(false);

  async function run(transition: Transition, save: boolean) {
    if (running.current) return;
    running.current = true;
    setSwitching(true);
    try {
      if (!save || await editor?.save()) await transition();
    } catch (error) {
      onError(error);
    } finally {
      running.current = false;
      setSwitching(false);
      setPending(null);
    }
  }

  function navigate(transition: Transition, protectDrafts = true) {
    if (running.current || pending) return Promise.resolve();
    if (protectDrafts && editor?.hasDrafts()) {
      setPending({ run: transition });
      return Promise.resolve();
    }
    return run(transition, false);
  }

  return {
    switching,
    pending: pending !== null,
    navigate,
    cancel: () => setPending(null),
    decide: (save: boolean) => {
      if (pending) void run(pending.run, save);
    },
  };
}
