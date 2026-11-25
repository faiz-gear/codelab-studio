import { useCallback, useEffect, useRef, useState } from "react";
import { expectResponse, type CoreClient } from "../core/client";
import type { AppCommand, CommandConfirmation, LearningProgress } from "../core/contracts";
import type { WorkspaceEditorSession } from "../workspace/editorSession";

export function useVerification(client: CoreClient | undefined, session: WorkspaceEditorSession | null, onError: (error: unknown) => void) {
  const [progress, setProgress] = useState<LearningProgress | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmation, setConfirmation] = useState<{chapterId: string; detail: CommandConfirmation} | null>(null);
  const version = useRef(0);
  const id = session?.workspace.id;
  useEffect(() => {
    if (!client || id === undefined) return;
    let active = true;
    let unsubscribe: (() => void) | undefined;
    const loadedAt = version.current;
    void client.subscribe(event => {
      if (active && event.type === "progressUpdated" && event.payload.workspaceId === id) {
        version.current++; setProgress(event.payload);
      }
    }).then(stop => { if (active) unsubscribe = stop; else stop(); }).catch(onError);
    void client.dispatch({type:"getProgress",payload:{workspaceId:id}}).then(response => {
      if (active && version.current === loadedAt) setProgress(expectResponse(response,"progress").payload);
    }).catch(onError);
    return () => { active = false; unsubscribe?.(); };
  },[client,id,onError]);
  const run = useCallback(async (chapterId: string, token?: number) => {
    if (!session || busy) return;
    setBusy(true); setConfirmation(null);
    const startedAt = version.current;
    try {
      const response = await session.runChecks(chapterId,token);
      if (response?.type === "commandConfirmationRequired") setConfirmation({chapterId,detail:response.payload});
      if (response?.type === "verificationStarted" && version.current === startedAt) setProgress(response.payload.progress);
    } catch(error) { onError(error); } finally { setBusy(false); }
  },[session,busy,onError]);
  const update = useCallback(async (command: AppCommand) => {
    if (!client) return;
    try { setProgress(expectResponse(await client.dispatch(command),"progress").payload); } catch(error) { onError(error); }
  },[client,onError]);
  async function exportRecord() {
    if (!client || id === undefined) return;
    try {
      const record = expectResponse(await client.dispatch({type:"exportLearningRecord",payload:{workspaceId:id}}),"learningRecord").payload;
      const url = URL.createObjectURL(new Blob([JSON.stringify(record,null,2)],{type:"application/json"}));
      const link = document.createElement("a"); link.href=url; link.download="codelab-learning-record.json"; link.click();
      setTimeout(()=>URL.revokeObjectURL(url),1000);
    } catch(error) { onError(error); }
  }
  return { progress: progress?.workspaceId === id ? progress : null, busy, confirmation, run, update, exportRecord, dismiss:()=>setConfirmation(null) };
}
