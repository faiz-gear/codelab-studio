import { useEffect, useState } from "react";
import { ChevronDownIcon, ChevronRightIcon, FileCodeIcon, FolderIcon } from "../components/Icons";
import type { Locale, WorkspaceEntry } from "../core/contracts";
import { formatDomainError, type Messages } from "../i18n";
import { domainError, type WorkspaceEditorSession } from "./editorSession";

interface TreeProps {
  session: WorkspaceEditorSession;
  version: number;
  activePath: string | null;
  onFile: (path: string) => void;
  messages: Messages["workspace"];
  locale: Locale;
}

export function WorkspaceTree(props: TreeProps) {
  return (
    <nav className="workspace-tree" aria-label={props.messages.files}>
      <Branch {...props} path="" />
    </nav>
  );
}

function Branch({ path, ...props }: TreeProps & { path: string }) {
  const { session, version, messages, locale, activePath, onFile } = props;
  const [entries, setEntries] = useState<WorkspaceEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void session.list(path).then((files) => {
      if (!current) return;
      setEntries(files);
      setError(null);
    }, (err: unknown) => {
      if (current) setError(formatDomainError(domainError(err), locale));
    });
    return () => { current = false; };
  }, [session, path, version, locale]);

  if (error) return <p role="alert" className="workspace-tree-status">{error}</p>;
  if (!entries) return <p role="status" className="workspace-tree-status">{messages.loading}</p>;
  if (!entries.length) return <p className="workspace-tree-status">{messages.emptyFolder}</p>;
  return (
    <ul className="workspace-tree-list">
      {entries.map((entry) => entry.kind === "directory" ? (
        <Directory key={entry.path} {...props} entry={entry} />
      ) : (
        <li key={entry.path}>
          <button
            className="workspace-tree-row mono"
            type="button"
            title={entry.path}
            aria-current={activePath === entry.path ? "true" : undefined}
            onClick={() => onFile(entry.path)}
          >
            <FileCodeIcon size={14} />
            <span>{entry.name}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}

function Directory({ entry, ...props }: TreeProps & { entry: WorkspaceEntry }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <li>
      <button
        className="workspace-tree-row mono"
        type="button"
        title={entry.path}
        aria-expanded={expanded}
        onClick={() => setExpanded((value) => !value)}
      >
        {expanded ? <ChevronDownIcon size={12} /> : <ChevronRightIcon size={12} />}
        <FolderIcon size={14} />
        <span>{entry.name}</span>
      </button>
      {expanded && <Branch {...props} path={entry.path} />}
    </li>
  );
}
