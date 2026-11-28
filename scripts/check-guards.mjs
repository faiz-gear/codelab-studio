#!/usr/bin/env node
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import postcss from "postcss";
import ts from "typescript";

const ROOT = resolve(import.meta.dirname, "..");
const failures = [];

function fail(rule, detail) {
  failures.push(`[${rule}] ${detail}`);
}

function readText(relPath) {
  return readFileSync(join(ROOT, relPath), "utf8");
}

function parseTypeScript(relPath) {
  return ts.createSourceFile(relPath, readText(relPath), ts.ScriptTarget.Latest);
}

function walkFiles(dirPath) {
  const out = [];
  for (const entry of readdirSync(dirPath)) {
    const full = join(dirPath, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      out.push(...walkFiles(full));
    } else {
      out.push(full);
    }
  }
  return out;
}

function pascalToScreamingSnake(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1_$2")
    .toUpperCase();
}

function pascalToCamel(name) {
  return name.charAt(0).toLowerCase() + name.slice(1);
}

function extractRustEnumVariants(source, enumName) {
  const regex = new RegExp(`pub\\s+enum\\s+${enumName}\\s*\\{([\\s\\S]*?)\\n\\}`);
  const match = regex.exec(source);
  if (!match) {
    fail("contract-parse", `Could not locate pub enum ${enumName} in Rust source.`);
    return [];
  }
  const variants = [];
  for (const rawLine of match[1].split("\n")) {
    const line = rawLine.trim();
    if (!line || line.startsWith("//") || line.startsWith("#[")) continue;
    if (line.startsWith("}") || line.includes(":")) continue;
    const m = /^([A-Z][A-Za-z0-9]*)\b/.exec(line);
    if (m) {
      variants.push(m[1]);
    }
  }
  return variants;
}

function extractTsUnionTags(source, typeName) {
  const regex = new RegExp(
    `export\\s+type\\s+${typeName}\\s*=([\\s\\S]*?);\\n\\n`,
    "m"
  );
  const match = regex.exec(source);
  if (!match) {
    fail("contract-parse", `Could not locate export type ${typeName} in contracts.ts.`);
    return [];
  }
  return Array.from(match[1].matchAll(/type:\s*"([A-Za-z0-9_]+)"/g), (m) => m[1]);
}

function assertSameSet(rule, label, leftName, leftItems, rightName, rightItems) {
  const left = new Set(leftItems);
  const right = new Set(rightItems);
  for (const item of left) {
    if (!right.has(item)) {
      fail(rule, `${label} "${item}" exists in ${leftName} but is missing in ${rightName}.`);
    }
  }
  for (const item of right) {
    if (!left.has(item)) {
      fail(rule, `${label} "${item}" exists in ${rightName} but is missing in ${leftName}.`);
    }
  }
}

// 1. ADR-0002: crates/codelab-core must remain 100% Tauri-free
const coreCargo = readText("crates/codelab-core/Cargo.toml");
if (/\btauri\b/i.test(coreCargo)) {
  fail(
    "adr-0002-core-purity",
    "crates/codelab-core/Cargo.toml must not depend on tauri crates."
  );
}

// 2. Frontend CoreClient seam: @tauri-apps/* only in src/core/tauriClient.ts, no __tests__ in prod
const sourceFiles = walkFiles(join(ROOT, "src")).map((file) =>
  relative(ROOT, file).replaceAll("\\", "/")
);
for (const rel of sourceFiles) {
  if (!rel.endsWith(".ts") && !rel.endsWith(".tsx")) continue;
  if (rel.startsWith("src/__tests__/")) continue;
  const content = readText(rel);

  if (rel !== "src/core/tauriClient.ts" && /@tauri-apps\//.test(content)) {
    fail(
      "core-client-seam",
      `${rel} imports @tauri-apps/* directly; route all IPC through CoreClient.`
    );
  }
  if (/["'][^"']*__tests__[^"']*["']/.test(content)) {
    fail(
      "no-test-double-in-prod",
      `${rel} imports from __tests__/; keep test doubles out of production modules.`
    );
  }
}

// 3. Rust <-> TypeScript <-> i18n contract parity
const rustErrorSrc = readText("crates/codelab-core/src/error.rs");
const rustAppSrc = readText("crates/codelab-core/src/app.rs");
const tsContractsSrc = readText("src/core/contracts.ts");

const rustErrorCodes = extractRustEnumVariants(rustErrorSrc, "ErrorCode").map(
  pascalToScreamingSnake
);
const tsErrorBlock = /export const ERROR_CODES = \[([\s\S]*?)\] as const;/.exec(
  tsContractsSrc
);
const tsErrorCodes = tsErrorBlock
  ? Array.from(tsErrorBlock[1].matchAll(/"([A-Z0-9_]+)"/g), (m) => m[1])
  : [];

// Frontend may define pure client transport codes (IPC_UNAVAILABLE, UNEXPECTED_ERROR)
const FRONTEND_ONLY_ERROR_CODES = new Set(["IPC_UNAVAILABLE", "UNEXPECTED_ERROR"]);
for (const code of rustErrorCodes) {
  if (!tsErrorCodes.includes(code)) {
    fail(
      "error-code-parity",
      `Rust ErrorCode "${code}" is missing from ERROR_CODES in src/core/contracts.ts.`
    );
  }
}
for (const code of tsErrorCodes) {
  if (!FRONTEND_ONLY_ERROR_CODES.has(code) && !rustErrorCodes.includes(code)) {
    fail(
      "error-code-parity",
      `TypeScript ErrorCode "${code}" is neither in Rust ErrorCode nor allowlisted as frontend-only.`
    );
  }
}

assertSameSet(
  "command-parity",
  "AppCommand",
  "Rust",
  extractRustEnumVariants(rustAppSrc, "AppCommand").map(pascalToCamel),
  "TypeScript",
  extractTsUnionTags(tsContractsSrc, "AppCommand")
);
assertSameSet(
  "response-parity",
  "AppResponse",
  "Rust",
  extractRustEnumVariants(rustAppSrc, "AppResponse").map(pascalToCamel),
  "TypeScript",
  extractTsUnionTags(tsContractsSrc, "AppResponse")
);
assertSameSet(
  "event-parity",
  "AppEvent",
  "Rust",
  extractRustEnumVariants(rustAppSrc, "AppEvent").map(pascalToCamel),
  "TypeScript",
  extractTsUnionTags(tsContractsSrc, "AppEvent")
);

// 3b. Compare placeholders in the actual locale dictionaries.
function propertyName(name) {
  return ts.isIdentifier(name) || ts.isStringLiteral(name) ? name.text : undefined;
}

function readErrorTemplates(relPath, dictionaryName) {
  const source = parseTypeScript(relPath);
  const dictionary = source.statements
    .filter(ts.isVariableStatement)
    .flatMap((statement) => statement.declarationList.declarations)
    .find((declaration) => ts.isIdentifier(declaration.name) && declaration.name.text === dictionaryName)
    ?.initializer;
  const templates = new Map();
  if (!dictionary || !ts.isObjectLiteralExpression(dictionary)) return templates;

  const errors = dictionary.properties
    .filter(ts.isPropertyAssignment)
    .find((property) => propertyName(property.name) === "errors")?.initializer;
  if (!errors || !ts.isObjectLiteralExpression(errors)) return templates;

  for (const property of errors.properties) {
    if (!ts.isPropertyAssignment(property) || !ts.isStringLiteralLike(property.initializer)) continue;
    const code = propertyName(property.name);
    if (code !== undefined) templates.set(code, property.initializer.text);
  }
  return templates;
}

function errorPlaceholders(templates, code, relPath) {
  const template = templates.get(code);
  if (template === undefined) {
    fail("i18n-error-coverage", `${relPath} is missing a string template for ${code}.`);
    return null;
  }

  // Match the keys accepted by src/i18n/index.ts#interpolate.
  const keys = Array.from(template.matchAll(/\{(\w+)\}/g), (match) => match[1]);
  if (keys.includes("reason")) {
    fail("i18n-no-raw-reason", `${relPath} ${code} renders {reason}. Use localized text for error reasons.`);
  }
  return keys;
}

const zhPath = "src/i18n/zh-CN.ts";
const enPath = "src/i18n/en-US.ts";
const zhTemplates = readErrorTemplates(zhPath, "zhCN");
const enTemplates = readErrorTemplates(enPath, "enUS");
for (const code of tsErrorCodes) {
  const zhKeys = errorPlaceholders(zhTemplates, code, zhPath);
  const enKeys = errorPlaceholders(enTemplates, code, enPath);
  if (zhKeys && enKeys) {
    assertSameSet("i18n-placeholder-parity", `${code} placeholder`, zhPath, zhKeys, enPath, enKeys);
  }
}

// 3b2. Rust <-> TypeScript <-> i18n ACP preset default command parity
function readTutorPresetLabels(relPath, dictionaryName) {
  const source = parseTypeScript(relPath);
  const dictionary = source.statements
    .filter(ts.isVariableStatement)
    .flatMap((statement) => statement.declarationList.declarations)
    .find((declaration) => ts.isIdentifier(declaration.name) && declaration.name.text === dictionaryName)
    ?.initializer;
  const labels = new Map();
  if (!dictionary || !ts.isObjectLiteralExpression(dictionary)) return labels;
  const tutor = dictionary.properties
    .filter(ts.isPropertyAssignment)
    .find((property) => propertyName(property.name) === "tutor")?.initializer;
  if (!tutor || !ts.isObjectLiteralExpression(tutor)) return labels;
  const presets = tutor.properties
    .filter(ts.isPropertyAssignment)
    .find((property) => propertyName(property.name) === "presets")?.initializer;
  if (!presets || !ts.isObjectLiteralExpression(presets)) return labels;
  for (const property of presets.properties) {
    if (!ts.isPropertyAssignment(property) || !ts.isStringLiteralLike(property.initializer)) continue;
    const id = propertyName(property.name);
    if (id !== undefined) labels.set(id, property.initializer.text);
  }
  return labels;
}

const rustTutorModelSrc = readText("crates/codelab-core/src/tutor/model.rs");
const rustDefaultCmdBlock =
  /pub fn default_command\(self\) -> &'static str \{([\s\S]*?)\n {4}\}/.exec(rustTutorModelSrc);
if (!rustDefaultCmdBlock) {
  fail("acp-preset-parity", "Could not locate AcpAgentPreset::default_command in crates/codelab-core/src/tutor/model.rs.");
} else {
  const rustPresets = new Map();
  for (const match of rustDefaultCmdBlock[1].matchAll(/Self::([A-Z][A-Za-z0-9]*)\s*=>\s*"([^"]*)"/g)) {
    rustPresets.set(pascalToCamel(match[1]), match[2]);
  }
  const tsPresetsBlock = /export const ACP_AGENT_PRESETS[\s\S]*?=\s*\[([\s\S]*?)\] as const;/.exec(tsContractsSrc);
  const tsPresets = new Map();
  if (tsPresetsBlock) {
    for (const match of tsPresetsBlock[1].matchAll(/id:\s*"([A-Za-z0-9]+)"[^}]*defaultCommand:\s*"([^"]*)"/g)) {
      tsPresets.set(match[1], match[2]);
    }
  }
  assertSameSet("acp-preset-parity", "AcpAgentPreset", "Rust", [...rustPresets.keys()], "TypeScript", [...tsPresets.keys()]);
  const zhPresetLabels = readTutorPresetLabels(zhPath, "zhCN");
  const enPresetLabels = readTutorPresetLabels(enPath, "enUS");
  for (const [id, rustCmd] of rustPresets) {
    const tsCmd = tsPresets.get(id);
    if (tsCmd !== rustCmd) {
      fail("acp-preset-parity", `Preset "${id}" default command mismatch: Rust has "${rustCmd}" vs TypeScript "${tsCmd}".`);
    }
    if (rustCmd) {
      if (!zhPresetLabels.get(id)?.includes(rustCmd)) {
        fail("acp-preset-parity", `${zhPath} tutor.presets.${id} does not show "${rustCmd}".`);
      }
      if (!enPresetLabels.get(id)?.includes(rustCmd)) {
        fail("acp-preset-parity", `${enPath} tutor.presets.${id} does not show "${rustCmd}".`);
      }
    }
  }
  const defaultTutorMatch =
    /export const DEFAULT_TUTOR_SETTINGS:\s*TutorSettings\s*=\s*\{([\s\S]*?)\};/.exec(tsContractsSrc);
  const defaultPreset = defaultTutorMatch && /preset:\s*"([A-Za-z0-9]+)"/.exec(defaultTutorMatch[1])?.[1];
  const defaultCommand = defaultTutorMatch && /command:\s*"([^"]*)"/.exec(defaultTutorMatch[1])?.[1];
  if (!defaultPreset || defaultCommand !== rustPresets.get(defaultPreset)) {
    fail(
      "acp-preset-parity",
      `DEFAULT_TUTOR_SETTINGS (${defaultPreset}: "${defaultCommand}") does not match Rust default "${rustPresets.get(defaultPreset)}".`
    );
  }
}

// 3c. Register every command called by the Tauri CoreClient.
const invokedCommands = new Set();
function collectInvokedCommands(node) {
  if (
    ts.isCallExpression(node) &&
    ts.isIdentifier(node.expression) &&
    node.expression.text === "invoke"
  ) {
    const command = node.arguments[0];
    if (command && ts.isStringLiteralLike(command)) invokedCommands.add(command.text);
  }
  ts.forEachChild(node, collectInvokedCommands);
}
collectInvokedCommands(parseTypeScript("src/core/tauriClient.ts"));

const shellLibSrc = readText("src-tauri/src/lib.rs").replace(
  /"(?:[^"\\]|\\.)*"|\/\/[^\r\n]*|\/\*[\s\S]*?\*\//g,
  " "
);
const handlerBlock = /\bgenerate_handler\s*!\s*\[([\s\S]*?)\]/.exec(shellLibSrc);
if (!handlerBlock) {
  fail("tauri-invoke-parity", "Could not locate tauri::generate_handler![...] in src-tauri/src/lib.rs.");
} else {
  const handlerNames = handlerBlock[1]
    .split(",")
    .map((handler) => handler.split("::").at(-1).trim())
    .filter(Boolean);
  const registeredHandlers = new Set(handlerNames);
  for (const command of invokedCommands) {
    if (!registeredHandlers.has(command)) {
      fail("tauri-invoke-parity", `src/core/tauriClient.ts calls "${command}" without a registered Tauri handler.`);
    }
  }
}

// 3d. Enforce CSS token integrity, Medium-Ceiling font-weight (<=500), gradient-token misuse, hero-mesh scope, and stylesheet manifest parity.
const appTokensPath = "src/styles/tokens.css";
const appTokens = readText(appTokensPath);
const tokensCss = postcss.parse(appTokens, { from: appTokensPath });
const gradientTokens = new Set();
const declaredCssTokens = new Set();
tokensCss.walkDecls(/^--/, (declaration) => {
  declaredCssTokens.add(declaration.prop);
  if (/\b(?:linear|radial|conic)-gradient\(/i.test(declaration.value)) {
    gradientTokens.add(declaration.prop);
  }
});

const cssFiles = sourceFiles.filter((rel) => rel.endsWith(".css"));
const parsedCssFiles = new Map();
for (const rel of cssFiles) {
  const css = rel === appTokensPath ? tokensCss : postcss.parse(readText(rel), { from: rel });
  parsedCssFiles.set(rel, css);
  if (rel !== appTokensPath) {
    css.walkDecls(/^--/, (declaration) => {
      declaredCssTokens.add(declaration.prop);
    });
  }
}

const COLOR_TYPED_PROPERTIES = new Set([
  "color",
  "border",
  "border-top",
  "border-right",
  "border-bottom",
  "border-left",
  "border-block",
  "border-block-start",
  "border-block-end",
  "border-inline",
  "border-inline-start",
  "border-inline-end",
  "outline",
  "box-shadow",
  "text-shadow",
  "text-decoration",
  "column-rule",
  "fill",
  "stroke",
]);
const HERO_MESH_TOKEN = "--arc-hero-bg";
const FONT_WEIGHT_CEILING_ALLOWLIST = new Set([".md-prose strong"]);
// Consume quoted text so only actual var() calls contribute references; capture optional fallback comma.
const CSS_VARIABLE = /"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|\bvar\(\s*(--[\w-]+)\s*(,|\))/gi;

for (const [rel, css] of parsedCssFiles) {
  css.walkDecls((declaration) => {
    const property = declaration.prop.toLowerCase();
    const location = `${rel}:${declaration.source.start.line}`;
    if (property === "font-weight") {
      const value = declaration.value.trim().toLowerCase();
      const numeric = Number(value);
      const exceedsCeiling =
        value === "bold" ||
        value === "bolder" ||
        (Number.isFinite(numeric) && numeric > 500);
      const selector = declaration.parent?.selector?.trim() ?? "";
      if (exceedsCeiling && !FONT_WEIGHT_CEILING_ALLOWLIST.has(selector)) {
        fail(
          "css-font-weight-ceiling",
          `${location} sets font-weight: ${declaration.value} on "${selector}"; DESIGN.md caps UI typography at 500.`
        );
      }
    }

    const expectsColor =
      COLOR_TYPED_PROPERTIES.has(property) ||
      (!property.startsWith("--") && property.endsWith("-color"));
    for (const [, token, delimiter] of declaration.value.matchAll(CSS_VARIABLE)) {
      if (!token) continue;
      if (delimiter !== "," && !declaredCssTokens.has(token)) {
        fail(
          "css-undefined-token",
          `${location} references undeclared CSS custom property ${token}.`
        );
      }
      if (expectsColor && gradientTokens.has(token)) {
        fail(
          "css-gradient-token-misuse",
          `${location} uses gradient token ${token} in color-typed property "${property}".`
        );
      }
      if (token === HERO_MESH_TOKEN && declaration.parent.selector !== ".hero-panel") {
        fail("css-hero-mesh-scope", `${location} uses ${token} outside the Welcome Home .hero-panel rule.`);
      }
    }
  });
}

// Ensure every stylesheet under src/styles/ is registered in src/styles/loadStyles.ts (shared by app & Playwright fixture).
const loadStylesSrc = readText("src/styles/loadStyles.ts");
const loadedStylesheets = new Set(
  Array.from(loadStylesSrc.matchAll(/import\s+["']\.\/([^"']+\.css)["']/g), (m) => `src/styles/${m[1]}`)
);
for (const rel of cssFiles) {
  if (rel.startsWith("src/styles/") && !loadedStylesheets.has(rel)) {
    fail(
      "css-stylesheet-manifest",
      `${rel} is not imported in src/styles/loadStyles.ts; register it so both the app and Playwright fixture load it.`
    );
  }
}

// 3e. Never Debug-format Rust ErrorCode variants into production strings.
function skipRustLiteral(source, i) {
  const prevIsIdent = i > 0 && /[A-Za-z0-9_]/.test(source[i - 1]);
  // Raw string literal: r"...", r#"..."#, br#"..."#, cr#"..."#
  const rawMatch = !prevIsIdent && /^(?:[bc]?r)(#*)"/.exec(source.slice(i));
  if (rawMatch) {
    const hashes = rawMatch[1];
    const pos = i + rawMatch[0].length;
    const endToken = `"${hashes}`;
    const closeIdx = source.indexOf(endToken, pos);
    return closeIdx === -1 ? source.length : closeIdx + endToken.length;
  }
  // Normal or byte/C string literal: "...", b"...", c"..."
  const strMatch = !prevIsIdent && /^[bc]?"/.exec(source.slice(i));
  if (strMatch) {
    let pos = i + strMatch[0].length;
    while (pos < source.length) {
      if (source[pos] === "\\") pos += 2;
      else if (source[pos] === "\"") return pos + 1;
      else pos += 1;
    }
    return pos;
  }
  // Character / byte-character literal (avoid consuming Rust lifetimes like 'a or 'static)
  const charMatch =
    !prevIsIdent &&
    /^b?'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^'\\])'/.exec(
      source.slice(i)
    );
  if (charMatch) {
    return i + charMatch[0].length;
  }
  return i;
}

function stripRustCommentsPreserveLines(source) {
  let out = "";
  let i = 0;
  while (i < source.length) {
    const ch = source[i];
    const next = source[i + 1];
    if (ch === "/" && next === "/") {
      while (i < source.length && source[i] !== "\n") {
        out += " ";
        i += 1;
      }
      continue;
    }
    if (ch === "/" && next === "*") {
      let depth = 1;
      out += "  ";
      i += 2;
      while (i < source.length && depth > 0) {
        if (source[i] === "/" && source[i + 1] === "*") {
          depth += 1;
          out += "  ";
          i += 2;
        } else if (source[i] === "*" && source[i + 1] === "/") {
          depth -= 1;
          out += "  ";
          i += 2;
        } else {
          out += source[i] === "\n" ? "\n" : " ";
          i += 1;
        }
      }
      continue;
    }
    const litEnd = skipRustLiteral(source, i);
    if (litEnd > i) {
      out += source.slice(i, litEnd);
      i = litEnd;
      continue;
    }
    out += ch;
    i += 1;
  }
  return out;
}

function stripCfgTestBlocksPreserveLines(source) {
  let out = source;
  const cfgTest = /#\[cfg\(test\)\]/g;
  let match;
  while ((match = cfgTest.exec(out)) !== null) {
    const start = match.index;
    let pos = cfgTest.lastIndex;
    let end = pos;
    while (pos < out.length) {
      const litEnd = skipRustLiteral(out, pos);
      if (litEnd > pos) {
        pos = litEnd;
        continue;
      }
      if (out[pos] === ";") {
        end = pos + 1;
        break;
      }
      if (out[pos] === "{") {
        let depth = 1;
        pos += 1;
        while (pos < out.length && depth > 0) {
          const innerLitEnd = skipRustLiteral(out, pos);
          if (innerLitEnd > pos) {
            pos = innerLitEnd;
            continue;
          }
          if (out[pos] === "{") depth += 1;
          else if (out[pos] === "}") depth -= 1;
          pos += 1;
        }
        end = pos;
        break;
      }
      pos += 1;
    }
    const blanked = out.slice(start, end).replace(/[^\n]/g, " ");
    out = out.slice(0, start) + blanked + out.slice(end);
    cfgTest.lastIndex = end;
  }
  return out;
}

function findRustFormatInvocations(source) {
  const results = [];
  const macroPattern = /\b(?:format|write|writeln)\s*!\s*([([{])/g;
  const closers = { "(": ")", "[": "]", "{": "}" };
  let match;
  while ((match = macroPattern.exec(source)) !== null) {
    const open = match[1];
    const close = closers[open];
    let depth = 1;
    let i = macroPattern.lastIndex;
    while (i < source.length && depth > 0) {
      const litEnd = skipRustLiteral(source, i);
      if (litEnd > i) {
        i = litEnd;
        continue;
      }
      if (source[i] === open) depth += 1;
      else if (source[i] === close) depth -= 1;
      i += 1;
    }
    const line = source.slice(0, match.index).split("\n").length;
    results.push({ line, body: source.slice(match.index, i) });
  }
  return results;
}

const rustCoreSrcFiles = walkFiles(join(ROOT, "crates/codelab-core/src")).map((file) =>
  relative(ROOT, file).replaceAll("\\", "/")
);
for (const rel of rustCoreSrcFiles) {
  if (!rel.endsWith(".rs") || rel === "crates/codelab-core/src/error.rs") continue;
  const cleaned = stripCfgTestBlocksPreserveLines(
    stripRustCommentsPreserveLines(readText(rel))
  );
  for (const { line, body } of findRustFormatInvocations(cleaned)) {
    const hasDebugSpec = /\{[^"{}]*:#?\?\}/.test(body);
    const referencesErrorCode =
      /\bErrorCode::/.test(body) ||
      /\.(?:error_)?code\b(?!\s*\()/.test(body) ||
      /\{\s*(?:err|error)_code\s*:#?\?\}/.test(body);
    if (hasDebugSpec && referencesErrorCode) {
      fail(
        "rust-no-debug-error-code",
        `${rel}:${line} Debug-formats an ErrorCode into a string; pass typed DomainError to UI or serialize via serde.`
      );
    }
  }
}

// 4. Design token sync: src/styles/tokens.css <-> docs/design/tokens.css
const docTokens = readText("docs/design/tokens.css");
if (appTokens !== docTokens) {
  fail(
    "tokens-sync",
    "src/styles/tokens.css and docs/design/tokens.css have drifted; keep them identical."
  );
}

if (failures.length > 0) {
  console.error("Architecture & contract guard checks failed:\n");
  for (const item of failures) {
    console.error(`  - ${item}`);
  }
  process.exit(1);
}

console.log("Architecture, contract, i18n, and token guards passed.");
