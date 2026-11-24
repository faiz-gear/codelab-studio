#!/usr/bin/env node
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";

const ROOT = resolve(import.meta.dirname, "..");
const failures = [];

function fail(rule, detail) {
  failures.push(`[${rule}] ${detail}`);
}

function readText(relPath) {
  return readFileSync(join(ROOT, relPath), "utf8");
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
for (const file of walkFiles(join(ROOT, "src"))) {
  const rel = relative(ROOT, file).replaceAll("\\", "/");
  if (!rel.endsWith(".ts") && !rel.endsWith(".tsx")) continue;
  if (rel.startsWith("src/__tests__/")) continue;
  const content = readFileSync(file, "utf8");

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
const zhSrc = readText("src/i18n/zh-CN.ts");
const enSrc = readText("src/i18n/en-US.ts");

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
  const keyRegex = new RegExp(`\\b${code}:\\s*"`);
  if (!keyRegex.test(zhSrc)) {
    fail("i18n-error-coverage", `src/i18n/zh-CN.ts is missing translation for ${code}.`);
  }
  if (!keyRegex.test(enSrc)) {
    fail("i18n-error-coverage", `src/i18n/en-US.ts is missing translation for ${code}.`);
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

// 4. Design token sync: src/styles/tokens.css <-> docs/design/tokens.css
const appTokens = readText("src/styles/tokens.css");
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
