#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");

const JS_EXTENSIONS = new Set([".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts"]);
const FORBIDDEN_CARGO_PLUGIN_RE = /\btauri[-_]plugin[-_](fs|shell|http)\b/g;
const FORBIDDEN_RUST_PLUGIN_RE = /\btauri_plugin_(fs|shell|http)\b/g;
const FORBIDDEN_CAPABILITY_PREFIX_RE = /^(fs|shell|http):/i;
const TAURI_APPS_IMPORT_RE = /(['"`])@tauri-apps\//;
const DANGEROUS_HTML_RE = /\bdangerouslySetInnerHTML\b/;
const DOMAIN_FORBIDDEN_LINE_PATTERNS = [
  { label: "tauri", re: /\btauri(?:_plugin_[a-z0-9_]+)?\b/ },
  { label: "tokio", re: /\btokio\b/ },
  { label: "reqwest", re: /\breqwest\b/ },
  { label: "std::fs", re: /\bstd\s*::\s*fs\b/ },
  { label: "std::process", re: /\bstd\s*::\s*process\b/ },
];

function toPosix(filePath) {
  return filePath.split(path.sep).join("/");
}

function relPosix(absPath) {
  return toPosix(path.relative(repoRoot, absPath));
}

function walkFiles(dirPath, predicate) {
  if (!fs.existsSync(dirPath)) {
    return [];
  }
  const results = [];
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });
  entries.sort((a, b) => a.name.localeCompare(b.name));
  for (const entry of entries) {
    const fullPath = path.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      results.push(...walkFiles(fullPath, predicate));
    } else if (entry.isFile() && predicate(fullPath)) {
      results.push(fullPath);
    }
  }
  return results;
}

function lineNumberAt(text, index) {
  let line = 1;
  for (let i = 0; i < index; i += 1) {
    if (text.charCodeAt(i) === 10) {
      line += 1;
    }
  }
  return line;
}

function stripJsComments(source) {
  const out = [];
  let state = "normal";
  for (let i = 0; i < source.length; i += 1) {
    const ch = source[i];
    const next = source[i + 1];

    if (state === "normal") {
      if (ch === "/" && next === "/") {
        state = "lineComment";
        out.push("  ");
        i += 1;
      } else if (ch === "/" && next === "*") {
        state = "blockComment";
        out.push("  ");
        i += 1;
      } else if (ch === "'") {
        state = "singleQuote";
        out.push(ch);
      } else if (ch === '"') {
        state = "doubleQuote";
        out.push(ch);
      } else if (ch === "`") {
        state = "template";
        out.push(ch);
      } else {
        out.push(ch);
      }
      continue;
    }

    if (state === "lineComment") {
      if (ch === "\n" || ch === "\r") {
        if (ch === "\n") {
          state = "normal";
        }
        out.push(ch);
      } else {
        out.push(" ");
      }
      continue;
    }

    if (state === "blockComment") {
      if (ch === "*" && next === "/") {
        state = "normal";
        out.push("  ");
        i += 1;
      } else if (ch === "\n" || ch === "\r") {
        out.push(ch);
      } else {
        out.push(" ");
      }
      continue;
    }

    if (state === "singleQuote" || state === "doubleQuote") {
      const quote = state === "singleQuote" ? "'" : '"';
      if (ch === "\\" && i + 1 < source.length) {
        out.push(ch, next);
        i += 1;
      } else if (ch === quote) {
        state = "normal";
        out.push(ch);
      } else if (ch === "\n") {
        state = "normal";
        out.push(ch);
      } else {
        out.push(ch);
      }
      continue;
    }

    if (state === "template") {
      if (ch === "\\" && i + 1 < source.length) {
        out.push(ch, next);
        i += 1;
      } else if (ch === "`") {
        state = "normal";
        out.push(ch);
      } else {
        out.push(ch);
      }
    }
  }
  return out.join("");
}

function isIdentChar(ch) {
  return ch !== undefined && /[A-Za-z0-9_]/.test(ch);
}

function tryMatchRustRawString(source, startIndex) {
  let idx = startIndex;
  const first = source[idx];
  if (first === "b" || first === "c") {
    if (source[idx + 1] !== "r") {
      return null;
    }
    idx += 1;
  } else if (first !== "r") {
    return null;
  }

  if (startIndex > 0 && isIdentChar(source[startIndex - 1])) {
    return null;
  }

  idx += 1;
  let hashes = 0;
  while (source[idx] === "#") {
    hashes += 1;
    idx += 1;
  }
  if (source[idx] !== '"') {
    return null;
  }

  return { openEnd: idx, hashes };
}

function stripRustCommentsAndStrings(source) {
  const out = [];
  let i = 0;
  while (i < source.length) {
    const ch = source[i];
    const next = source[i + 1];

    if (ch === "/" && next === "/") {
      out.push("  ");
      i += 2;
      while (i < source.length && source[i] !== "\n") {
        out.push(source[i] === "\r" ? "\r" : " ");
        i += 1;
      }
      continue;
    }

    if (ch === "/" && next === "*") {
      let depth = 1;
      out.push("  ");
      i += 2;
      while (i < source.length && depth > 0) {
        if (source[i] === "/" && source[i + 1] === "*") {
          depth += 1;
          out.push("  ");
          i += 2;
        } else if (source[i] === "*" && source[i + 1] === "/") {
          depth -= 1;
          out.push("  ");
          i += 2;
        } else {
          out.push(source[i] === "\n" || source[i] === "\r" ? source[i] : " ");
          i += 1;
        }
      }
      continue;
    }

    const rawStart = tryMatchRustRawString(source, i);
    if (rawStart !== null) {
      while (i <= rawStart.openEnd) {
        out.push(" ");
        i += 1;
      }
      while (i < source.length) {
        if (source[i] === '"') {
          let matchedHashes = 0;
          while (
            matchedHashes < rawStart.hashes &&
            source[i + 1 + matchedHashes] === "#"
          ) {
            matchedHashes += 1;
          }
          if (matchedHashes === rawStart.hashes) {
            for (let k = 0; k <= rawStart.hashes; k += 1) {
              out.push(" ");
            }
            i += 1 + rawStart.hashes;
            break;
          }
        }
        out.push(source[i] === "\n" || source[i] === "\r" ? source[i] : " ");
        i += 1;
      }
      continue;
    }

    if (ch === '"') {
      out.push(" ");
      i += 1;
      while (i < source.length) {
        const cur = source[i];
        if (cur === "\\" && i + 1 < source.length) {
          out.push(" ");
          out.push(source[i + 1] === "\n" || source[i + 1] === "\r" ? source[i + 1] : " ");
          i += 2;
        } else if (cur === '"') {
          out.push(" ");
          i += 1;
          break;
        } else {
          out.push(cur === "\n" || cur === "\r" ? cur : " ");
          i += 1;
        }
      }
      continue;
    }

    if (ch === "'") {
      if (next === "\\") {
        let j = i + 2;
        while (j < source.length && source[j] !== "'" && source[j] !== "\n") {
          j += 1;
        }
        if (j < source.length && source[j] === "'") {
          for (let k = i; k <= j; k += 1) {
            out.push(" ");
          }
          i = j + 1;
          continue;
        }
      } else if (
        next !== undefined &&
        next !== "'" &&
        next !== "\n" &&
        source[i + 2] === "'"
      ) {
        out.push("   ");
        i += 3;
        continue;
      }
    }

    out.push(ch);
    i += 1;
  }
  return out.join("");
}

function stripTomlLineComment(line) {
  let inDouble = false;
  let inSingle = false;
  for (let i = 0; i < line.length; i += 1) {
    const ch = line[i];
    if (inDouble) {
      if (ch === "\\") {
        i += 1;
      } else if (ch === '"') {
        inDouble = false;
      }
    } else if (inSingle) {
      if (ch === "'") {
        inSingle = false;
      }
    } else if (ch === '"') {
      inDouble = true;
    } else if (ch === "'") {
      inSingle = true;
    } else if (ch === "#") {
      return line.slice(0, i);
    }
  }
  return line;
}

function findJsonStringLine(lines, needle, startIndex = 0) {
  const quoted = JSON.stringify(needle);
  for (let i = startIndex; i < lines.length; i += 1) {
    if (lines[i].includes(quoted)) {
      return i + 1;
    }
  }
  return 1;
}

function checkRule1FrontendIpcBoundary() {
  const srcDir = path.join(repoRoot, "src");
  const files = walkFiles(srcDir, (f) => JS_EXTENSIONS.has(path.extname(f)));
  const violations = [];

  for (const absPath of files) {
    const rel = relPosix(absPath);
    if (rel.startsWith("src/lib/ipc/")) {
      continue;
    }
    const stripped = stripJsComments(fs.readFileSync(absPath, "utf8"));
    const lines = stripped.split(/\r?\n/);
    for (let i = 0; i < lines.length; i += 1) {
      if (TAURI_APPS_IMPORT_RE.test(lines[i])) {
        violations.push({
          file: rel,
          line: i + 1,
          message: "Direct '@tauri-apps/' import is forbidden outside src/lib/ipc/",
        });
      }
    }
  }

  return {
    id: 1,
    name: "Frontend IPC boundary (no @tauri-apps/ outside src/lib/ipc/)",
    detail: `scanned ${files.length} files`,
    violations,
  };
}

function checkRule2NoDangerousHtml() {
  const srcDir = path.join(repoRoot, "src");
  const files = walkFiles(srcDir, (f) => JS_EXTENSIONS.has(path.extname(f)));
  const violations = [];

  for (const absPath of files) {
    const rel = relPosix(absPath);
    const stripped = stripJsComments(fs.readFileSync(absPath, "utf8"));
    const lines = stripped.split(/\r?\n/);
    for (let i = 0; i < lines.length; i += 1) {
      if (DANGEROUS_HTML_RE.test(lines[i])) {
        violations.push({
          file: rel,
          line: i + 1,
          message: "dangerouslySetInnerHTML is forbidden under src/",
        });
      }
    }
  }

  return {
    id: 2,
    name: "No dangerouslySetInnerHTML under src/",
    detail: `scanned ${files.length} files`,
    violations,
  };
}

function checkRule3ForbiddenTauriPlugins() {
  const violations = [];

  const cargoPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
  if (!fs.existsSync(cargoPath)) {
    violations.push({
      file: "src-tauri/Cargo.toml",
      line: 1,
      message: "Required file src-tauri/Cargo.toml is missing",
    });
  } else {
    const lines = fs.readFileSync(cargoPath, "utf8").split(/\r?\n/);
    for (let i = 0; i < lines.length; i += 1) {
      const code = stripTomlLineComment(lines[i]);
      FORBIDDEN_CARGO_PLUGIN_RE.lastIndex = 0;
      const match = FORBIDDEN_CARGO_PLUGIN_RE.exec(code);
      if (match) {
        violations.push({
          file: "src-tauri/Cargo.toml",
          line: i + 1,
          message: `Forbidden Tauri plugin dependency '${match[0]}'`,
        });
      }
    }
  }

  const libPath = path.join(repoRoot, "src-tauri", "src", "lib.rs");
  if (!fs.existsSync(libPath)) {
    violations.push({
      file: "src-tauri/src/lib.rs",
      line: 1,
      message: "Required file src-tauri/src/lib.rs is missing",
    });
  } else {
    const stripped = stripRustCommentsAndStrings(fs.readFileSync(libPath, "utf8"));
    const lines = stripped.split(/\r?\n/);
    for (let i = 0; i < lines.length; i += 1) {
      FORBIDDEN_RUST_PLUGIN_RE.lastIndex = 0;
      const match = FORBIDDEN_RUST_PLUGIN_RE.exec(lines[i]);
      if (match) {
        violations.push({
          file: "src-tauri/src/lib.rs",
          line: i + 1,
          message: `Forbidden Tauri plugin registration '${match[0]}'`,
        });
      }
    }
  }

  const capabilitiesDir = path.join(repoRoot, "src-tauri", "capabilities");
  const capabilityFiles = walkFiles(capabilitiesDir, (f) => f.endsWith(".json"));
  if (capabilityFiles.length === 0) {
    violations.push({
      file: "src-tauri/capabilities",
      line: 1,
      message: "No capability JSON files found under src-tauri/capabilities/",
    });
  }

  for (const absPath of capabilityFiles) {
    const rel = relPosix(absPath);
    const raw = fs.readFileSync(absPath, "utf8");
    const lines = raw.split(/\r?\n/);
    let parsed;
    try {
      parsed = JSON.parse(raw);
    } catch (err) {
      violations.push({
        file: rel,
        line: 1,
        message: `Invalid JSON in capability file: ${err.message}`,
      });
      continue;
    }

    const permissions = Array.isArray(parsed.permissions) ? parsed.permissions : [];
    let searchCursor = 0;
    for (const entry of permissions) {
      const identifier =
        typeof entry === "string"
          ? entry
          : entry && typeof entry === "object" && typeof entry.identifier === "string"
            ? entry.identifier
            : null;
      if (!identifier) {
        continue;
      }
      const line = findJsonStringLine(lines, identifier, searchCursor);
      searchCursor = Math.max(searchCursor, line - 1);
      if (FORBIDDEN_CAPABILITY_PREFIX_RE.test(identifier)) {
        violations.push({
          file: rel,
          line,
          message: `Forbidden capability permission '${identifier}'`,
        });
      }
    }
  }

  return {
    id: 3,
    name: "No forbidden Tauri plugins or capabilities (fs/shell/http)",
    detail: `checked Cargo.toml, lib.rs, and ${capabilityFiles.length} capability file(s)`,
    violations,
  };
}

function checkRule4PureDomainLayer() {
  const domainDir = path.join(repoRoot, "src-tauri", "src", "domain");
  const files = walkFiles(domainDir, (f) => f.endsWith(".rs"));
  const violations = [];

  if (files.length === 0) {
    violations.push({
      file: "src-tauri/src/domain",
      line: 1,
      message: "No Rust domain files found under src-tauri/src/domain/",
    });
  }

  for (const absPath of files) {
    const rel = relPosix(absPath);
    const stripped = stripRustCommentsAndStrings(fs.readFileSync(absPath, "utf8"));
    const lines = stripped.split(/\r?\n/);

    for (let i = 0; i < lines.length; i += 1) {
      const line = lines[i];
      for (const pattern of DOMAIN_FORBIDDEN_LINE_PATTERNS) {
        if (pattern.re.test(line)) {
          violations.push({
            file: rel,
            line: i + 1,
            message: `Forbidden dependency reference '${pattern.label}' inside pure domain module`,
          });
        }
      }
    }

    const groupedStdStartRe = /\bstd\s*::\s*\{/g;
    let groupMatch;
    while ((groupMatch = groupedStdStartRe.exec(stripped)) !== null) {
      const openBrace = groupMatch.index + groupMatch[0].length - 1;
      let depth = 1;
      let cursor = openBrace + 1;
      while (cursor < stripped.length && depth > 0) {
        if (stripped[cursor] === "{") {
          depth += 1;
        } else if (stripped[cursor] === "}") {
          depth -= 1;
        }
        cursor += 1;
      }
      const inner = stripped.slice(openBrace + 1, cursor - 1);
      const innerOffset = openBrace + 1;
      const memberRe = /\b(fs|process)\b/g;
      let memberMatch;
      while ((memberMatch = memberRe.exec(inner)) !== null) {
        const line = lineNumberAt(stripped, innerOffset + memberMatch.index);
        violations.push({
          file: rel,
          line,
          message: `Forbidden grouped import 'std::${memberMatch[1]}' inside pure domain module`,
        });
      }
    }
  }

  return {
    id: 4,
    name: "Pure domain layer (no tauri/tokio/std::fs/std::process/reqwest in domain/)",
    detail: `scanned ${files.length} Rust file(s)`,
    violations,
  };
}

function checkRule5TokensSync() {
  const sourcePath = path.join(repoRoot, "docs", "design", "tokens.css");
  const targetPath = path.join(repoRoot, "src", "styles", "tokens.css");
  const violations = [];

  if (!fs.existsSync(sourcePath)) {
    violations.push({
      file: "docs/design/tokens.css",
      line: 1,
      message: "Source token file docs/design/tokens.css is missing",
    });
  }
  if (!fs.existsSync(targetPath)) {
    violations.push({
      file: "src/styles/tokens.css",
      line: 1,
      message: "Target token file src/styles/tokens.css is missing",
    });
  }

  let byteCount = 0;
  if (violations.length === 0) {
    const sourceBytes = fs.readFileSync(sourcePath);
    const targetBytes = fs.readFileSync(targetPath);
    byteCount = sourceBytes.length;
    if (!sourceBytes.equals(targetBytes)) {
      const limit = Math.min(sourceBytes.length, targetBytes.length);
      let firstDiff = limit;
      for (let i = 0; i < limit; i += 1) {
        if (sourceBytes[i] !== targetBytes[i]) {
          firstDiff = i;
          break;
        }
      }
      let line = 1;
      for (let i = 0; i < Math.min(firstDiff, targetBytes.length); i += 1) {
        if (targetBytes[i] === 0x0a) {
          line += 1;
        }
      }
      violations.push({
        file: "src/styles/tokens.css",
        line,
        message: `Byte drift against docs/design/tokens.css at byte ${firstDiff} (${sourceBytes.length} vs ${targetBytes.length} bytes)`,
      });
    }
  }

  return {
    id: 5,
    name: "tokens.css byte-identical sync (src/styles/tokens.css == docs/design/tokens.css)",
    detail: `${byteCount} bytes identical`,
    violations,
  };
}

const results = [
  checkRule1FrontendIpcBoundary(),
  checkRule2NoDangerousHtml(),
  checkRule3ForbiddenTauriPlugins(),
  checkRule4PureDomainLayer(),
  checkRule5TokensSync(),
];

let totalViolations = 0;
for (const result of results) {
  if (result.violations.length === 0) {
    console.log(`PASS [Rule ${result.id}] ${result.name} (${result.detail})`);
  } else {
    totalViolations += result.violations.length;
    console.error(`FAIL [Rule ${result.id}] ${result.name}`);
    for (const violation of result.violations) {
      console.error(`  - ${violation.file}:${violation.line}: ${violation.message}`);
    }
  }
}

if (totalViolations > 0) {
  console.error(`Architecture guards failed with ${totalViolations} violation(s).`);
  process.exit(1);
}

console.log("All 5 architecture guards passed.");
