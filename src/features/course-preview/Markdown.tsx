import { Fragment, type ReactNode } from "react";
import type { MarkdownNode } from "../../lib/ipc/generated";

export function Markdown({ nodes }: { nodes: readonly MarkdownNode[] }) {
  return nodes.map((node, index) => <Fragment key={index}>{renderNode(node)}</Fragment>);
}

function renderNode(node: MarkdownNode): ReactNode {
  switch (node.kind) {
    case "paragraph":
      return <p><Markdown nodes={node.children} /></p>;
    case "heading": {
      const Heading = node.level <= 3 ? "h3" : node.level === 4 ? "h4" : node.level === 5 ? "h5" : "h6";
      return <Heading id={node.id ? `manual-${node.id}` : undefined}><Markdown nodes={node.children} /></Heading>;
    }
    case "text":
      return node.value;
    case "strong":
      return <strong><Markdown nodes={node.children} /></strong>;
    case "emphasis":
      return <em><Markdown nodes={node.children} /></em>;
    case "inlineCode":
      return <code>{node.value}</code>;
    case "codeBlock":
      return <pre><code>{node.code}</code></pre>;
    case "list": {
      const items = node.items.map((item, index) => <li key={index}><Markdown nodes={item} /></li>);
      return node.ordered ? <ol start={node.start ?? undefined}>{items}</ol> : <ul>{items}</ul>;
    }
    case "blockQuote":
      return <blockquote><Markdown nodes={node.children} /></blockquote>;
    case "table":
      return (
        <div className="manual-table" tabIndex={0} role="region" aria-label="Table">
          <table>
            <thead><tr>{node.headers.map((cell, index) => <th key={index} scope="col"><Markdown nodes={cell} /></th>)}</tr></thead>
            <tbody>{node.rows.map((row, index) => (
              <tr key={index}>{row.map((cell, col) => <td key={col}><Markdown nodes={cell} /></td>)}</tr>
            ))}</tbody>
          </table>
        </div>
      );
    case "link":
      return node.href.startsWith("#") ? (
        <a href={`#manual-${node.href.slice(1)}`} title={node.title ?? undefined}><Markdown nodes={node.children} /></a>
      ) : (
        <span title={node.href}><Markdown nodes={node.children} /></span>
      );
    case "image":
      return <span title={node.title ?? node.src}>{node.alt || node.src}</span>;
    case "lineBreak":
      return <br />;
    case "thematicBreak":
      return <hr />;
    case "kbd":
      return <kbd>{node.label}</kbd>;
    case "superscript":
      return <sup><Markdown nodes={node.children} /></sup>;
    case "subscript":
      return <sub><Markdown nodes={node.children} /></sub>;
    case "details":
      return <details><summary>{node.summary}</summary><Markdown nodes={node.children} /></details>;
    default: {
      const unhandled: never = node;
      return unhandled;
    }
  }
}
