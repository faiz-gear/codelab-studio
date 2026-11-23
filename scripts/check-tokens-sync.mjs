import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");

const sourcePath = path.join(repoRoot, "docs", "design", "tokens.css");
const targetPath = path.join(repoRoot, "src", "styles", "tokens.css");

const sourceBytes = readFileSync(sourcePath);
const targetBytes = readFileSync(targetPath);

if (!sourceBytes.equals(targetBytes)) {
  const limit = Math.min(sourceBytes.length, targetBytes.length);
  let firstDiff = limit;
  for (let i = 0; i < limit; i += 1) {
    if (sourceBytes[i] !== targetBytes[i]) {
      firstDiff = i;
      break;
    }
  }
  console.error(
    `tokens.css drift detected: docs/design/tokens.css (${sourceBytes.length} bytes) !== src/styles/tokens.css (${targetBytes.length} bytes), first diff at byte ${firstDiff}.`,
  );
  process.exit(1);
}

console.log(`tokens.css sync OK (${sourceBytes.length} bytes identical).`);
