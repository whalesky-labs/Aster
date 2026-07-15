import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const baseline = JSON.parse(
  fs.readFileSync(path.join(root, "config/engineering-baseline.json"), "utf8"),
);
const sourceRoots = ["src", "src-tauri/src"];
const sourceExtensions = new Set([".ts", ".tsx", ".css", ".rs"]);
const failures = [];

function walk(directory) {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const absolute = path.join(directory, entry.name);
    return entry.isDirectory() ? walk(absolute) : [absolute];
  });
}

function relative(file) {
  return path.relative(root, file).split(path.sep).join("/");
}

function lineCount(text) {
  return text.length === 0 ? 0 : text.replace(/\r?\n$/, "").split(/\r?\n/).length;
}

function productionRustPanicCount(file, text) {
  if (!file.endsWith(".rs")) return 0;
  if (file.includes("/tests/") || file.endsWith("/tests.rs") || file.endsWith("/test_support.rs")) return 0;
  const production = text.split(/\n\s*#\[cfg\(test\)\]/, 1)[0];
  return [...production.matchAll(/\.(?:unwrap|expect)\s*\(|\bpanic!\s*\(/g)].length;
}

function isTestSource(file) {
  return file.includes("/tests/") || file.endsWith("/tests.rs") || file.endsWith("/test_support.rs");
}

for (const sourceRoot of sourceRoots) {
  for (const absolute of walk(path.join(root, sourceRoot))) {
    if (!sourceExtensions.has(path.extname(absolute))) continue;
    const file = relative(absolute);
    const text = fs.readFileSync(absolute, "utf8");
    const lines = lineCount(text);
    const limit = baseline.maxNewSourceFileLines;
    if (!isTestSource(file) && lines > limit) {
      failures.push(`${file}: ${lines} production lines exceeds ${limit}`);
    }

    const panicCount = productionRustPanicCount(file, text);
    const panicLimit = baseline.legacyRustPanicMacroCounts[file] ?? 0;
    if (panicCount > panicLimit) {
      failures.push(
        `${file}: ${panicCount} production unwrap/expect/panic uses exceeds ${panicLimit}`,
      );
    }
  }
}

for (const file of walk(path.join(root, "src"))) {
  if (!new Set([".ts", ".tsx"]).has(path.extname(file))) continue;
  const relativeFile = relative(file);
  const text = fs.readFileSync(file, "utf8");
  if (/\bany\b/.test(text) && !isTestSource(relativeFile)) {
    failures.push(`${relativeFile}: new modules cannot use the any type`);
  }
  if (relativeFile.startsWith("src/entities/") && /from ["'](?:react|@tauri-apps)/.test(text)) {
    failures.push(`${relativeFile}: entities cannot depend on React or Tauri`);
  }
  if (relativeFile.startsWith("src/shared/") && /from ["']\.\.\/\.\.\/features\//.test(text)) {
    failures.push(`${relativeFile}: shared cannot depend on features`);
  }
}

if (failures.length > 0) {
  console.error("Engineering standards check failed:\n");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log("Engineering standards check passed.");
