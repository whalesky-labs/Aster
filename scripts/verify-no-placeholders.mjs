import { existsSync, readdirSync, readFileSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { arch, platform, release } from "node:os";

const root = process.cwd();
const scannedRoots = [
  "README.md",
  "docs/ASTER_EXECUTION_PLAN.md",
  "docs/ASTER_CROSS_PLATFORM_ACCEPTANCE.md",
  "docs/ASTER_DISASTER_RECOVERY_RUNBOOK.md",
  "docs/manual-acceptance/README.md",
  "scripts",
  "src",
  "src-tauri/src",
  "src-tauri/migrations",
];

const forbiddenPatterns = [
  /\bTODO\b/i,
  /\bFIXME\b/i,
  /\btodo!\s*\(/i,
  /\bunimplemented!\s*\(/i,
  /\bmock\b/i,
  /\bstub\b/i,
  /待实现/,
  /未实现/,
  /假实现/,
  /占位/,
];

const allowed = [
  (relativePath, line) => relativePath === "scripts/verify-no-placeholders.mjs" && line.includes("forbiddenPatterns"),
  (relativePath, line) => relativePath === "scripts/verify-no-placeholders.mjs" && /^\/.*\/,?$/.test(line.trim()),
  (relativePath, line) => relativePath === "scripts/verify-execution-coverage.mjs" && line.includes("verify:no-placeholders"),
  (relativePath, line) => relativePath === "scripts/verify-execution-coverage.mjs" && line.includes("no-placeholder-scan-${process.platform}"),
  (relativePath, line) => relativePath === "scripts/verify-execution-coverage.mjs" && line.includes("no-placeholder-scan-${evidencePlatform}"),
  (relativePath, line) => relativePath === "scripts/verify-execution-coverage.mjs" && line.includes("no-placeholder-scan 最新证据状态必须为 passed"),
  (relativePath, line) => relativePath === "scripts/verify-execution-coverage.mjs" && line.includes("setupRejectedAcceptancePackageFixture"),
  (relativePath) => relativePath === "scripts/test-manual-acceptance-paths.mjs",
  (relativePath, line) => relativePath === "src/App.tsx" && line.includes("placeholder="),
  (relativePath, line) => relativePath === "src/App.css" && line.includes("placeholder-panel"),
  (relativePath, line) => relativePath === "src/App.tsx" && line.includes("placeholder-panel"),
  (relativePath, line) => line.includes("临时断开"),
  (relativePath, line) => line.includes("临时登录"),
];

function collectFiles(path, relativePath = path) {
  const fullPath = join(root, path);
  if (!existsSync(fullPath)) return [];
  const stat = statSync(fullPath);
  if (stat.isFile()) return [relativePath.replaceAll("\\", "/")];
  return readdirSync(fullPath, { withFileTypes: true }).flatMap((entry) => {
    const nextRelativePath = `${relativePath.replaceAll("\\", "/")}/${entry.name}`;
    if (entry.isDirectory()) return collectFiles(nextRelativePath, nextRelativePath);
    return [nextRelativePath];
  });
}

const files = scannedRoots.flatMap((path) => collectFiles(path));
const findings = [];

for (const relativePath of files) {
  if (!/\.(css|json|md|mjs|rs|sql|tsx|ts)$/.test(relativePath)) continue;
  const text = readFileSync(join(root, relativePath), "utf8");
  text.split(/\r?\n/).forEach((line, index) => {
    for (const pattern of forbiddenPatterns) {
      if (!pattern.test(line)) continue;
      if (allowed.some((predicate) => predicate(relativePath, line))) continue;
      findings.push({
        path: relativePath,
        line: index + 1,
        pattern: pattern.source,
        text: line.trim(),
      });
    }
  });
}

mkdirSync(join(root, "docs", "release-evidence"), { recursive: true });
const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const reportPath = join(root, "docs", "release-evidence", `no-placeholder-scan-${process.platform}-${timestamp}.json`);
const report = {
  generatedAt: new Date().toISOString(),
  platform: platform(),
  platformRelease: release(),
  arch: arch(),
  status: findings.length === 0 ? "passed" : "failed",
  scannedRoots,
  findings,
};
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);

if (findings.length > 0) {
  console.error("[verify-no-placeholders] Found placeholder or unfinished markers:");
  for (const finding of findings) {
    console.error(`- ${finding.path}:${finding.line} ${finding.text}`);
  }
  console.error(`[verify-no-placeholders] Report: ${reportPath}`);
  process.exit(1);
}

console.log("[verify-no-placeholders] No unfinished placeholder markers found.");
console.log(`[verify-no-placeholders] Report: ${reportPath}`);
