#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");

function buildEnv() {
  const env = { ...process.env };
  const cargoBin = path.join(os.homedir(), ".cargo", "bin");
  if (fs.existsSync(cargoBin)) {
    const pathKey = Object.keys(env).find((key) => key.toLowerCase() === "path") ?? "PATH";
    const current = env[pathKey] ?? "";
    const parts = current.split(path.delimiter).filter(Boolean);
    if (!parts.includes(cargoBin)) {
      env[pathKey] = current ? `${cargoBin}${path.delimiter}${current}` : cargoBin;
    }
  }
  return env;
}

function runCommand(command, args, env, capture = false) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    env,
    encoding: "utf8",
    stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
  });
  if (result.error) {
    console.error(`Failed to execute '${command}': ${result.error.message}`);
    process.exit(1);
  }
  return result;
}

function snapshotContracts() {
  const snapshot = new Map();
  const generatedPath = path.join(repoRoot, "src", "lib", "ipc", "generated.ts");
  if (fs.existsSync(generatedPath)) {
    snapshot.set("src/lib/ipc/generated.ts", fs.readFileSync(generatedPath));
  }
  const schemasDir = path.join(repoRoot, "schemas");
  if (fs.existsSync(schemasDir)) {
    const entries = fs
      .readdirSync(schemasDir, { withFileTypes: true })
      .filter((entry) => entry.isFile())
      .sort((a, b) => a.name.localeCompare(b.name));
    for (const entry of entries) {
      snapshot.set(`schemas/${entry.name}`, fs.readFileSync(path.join(schemasDir, entry.name)));
    }
  }
  return snapshot;
}

function findSnapshotDrift(before, after) {
  const keys = new Set([...before.keys(), ...after.keys()]);
  const drifted = [];
  for (const key of [...keys].sort()) {
    const prev = before.get(key);
    const next = after.get(key);
    if (!prev) {
      drifted.push(`${key}: created by export_contracts (${next.length} bytes)`);
    } else if (!next) {
      drifted.push(`${key}: missing after export_contracts`);
    } else if (!prev.equals(next)) {
      drifted.push(`${key}: content drifted (${prev.length} bytes before vs ${next.length} bytes generated)`);
    }
  }
  return drifted;
}

const env = buildEnv();
const beforeExport = snapshotContracts();

const exportRun = runCommand(
  "cargo",
  ["test", "--manifest-path", "src-tauri/Cargo.toml", "--test", "export_contracts"],
  env,
);
if (exportRun.status !== 0) {
  process.exit(exportRun.status ?? 1);
}

const afterExport = snapshotContracts();
const snapshotDrift = findSnapshotDrift(beforeExport, afterExport);

const diffRun = runCommand(
  "git",
  ["diff", "--exit-code", "--", "src/lib/ipc/generated.ts", "schemas/"],
  env,
);

const statusRun = runCommand(
  "git",
  ["status", "--porcelain", "--", "src/lib/ipc/generated.ts", "schemas/"],
  env,
  true,
);
if (statusRun.status !== 0) {
  console.error(statusRun.stderr || "git status check failed");
  process.exit(statusRun.status ?? 1);
}

if (snapshotDrift.length > 0 || diffRun.status !== 0 || statusRun.stdout.trim().length > 0) {
  if (snapshotDrift.length > 0) {
    console.error("Contract drift detected between checked-in files and export_contracts output:");
    for (const item of snapshotDrift) {
      console.error(`  - ${item}`);
    }
  }
  if (statusRun.stdout.trim().length > 0) {
    console.error("Uncommitted contract file changes:\n" + statusRun.stdout.trim());
  }
  console.error(
    "Regenerate contracts with 'cargo test --manifest-path src-tauri/Cargo.toml --test export_contracts' and commit the results.",
  );
  process.exit(1);
}

console.log("Contract export drift check OK (src/lib/ipc/generated.ts and schemas/ are up to date).");
