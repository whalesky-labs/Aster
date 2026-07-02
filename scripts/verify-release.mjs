import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join, relative } from "node:path";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { arch, platform, release } from "node:os";

const root = process.cwd();
const verifiedArtifacts = [];
const commandResults = [];
const tauriConfigPath = join(root, "src-tauri", "tauri.conf.json");
const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, "utf8"));
const tauriBuildTarget = process.env.TAURI_BUILD_TARGET?.trim() || "";
let resolvedBundleDir = null;
const requiredReleaseCommands = [
  "npm run build",
  "npm run verify:coverage",
  "npm run verify:no-placeholders",
  "cargo fmt --check",
  "cargo test",
  tauriBuildTarget ? `npm run tauri -- build --target ${tauriBuildTarget}` : "npm run tauri -- build",
];

function run(command, args, options = {}) {
  const commandText = [command, ...args].join(" ");
  const cwd = options.cwd ?? root;
  const startedAt = new Date();
  console.log(`\n$ ${commandText}`);
  const result = spawnSync(command, args, {
    cwd,
    env: process.env,
    shell: process.platform === "win32",
    stdio: "inherit",
  });
  const finishedAt = new Date();
  const commandResult = {
    command: commandText,
    cwd,
    status: result.status === 0 ? "passed" : "failed",
    exitCode: result.status,
    signal: result.signal ?? null,
    startedAt: startedAt.toISOString(),
    finishedAt: finishedAt.toISOString(),
    durationMs: finishedAt.getTime() - startedAt.getTime(),
  };
  commandResults.push(commandResult);
  if (result.status !== 0) {
    writeEvidenceReport("failed", {
      failedCommand: commandResult,
    });
    process.exit(result.status ?? 1);
  }
}

function capture(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? root,
    env: process.env,
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  if (result.status !== 0) {
    return null;
  }
  return result.stdout.trim();
}

function collectFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? collectFiles(path) : [path];
  });
}

function assertFile(path, message) {
  if (!existsSync(path)) {
    console.error(`\n[verify-release] ${message}: ${path}`);
    process.exit(1);
  }
}

function appleDoubleFiles(dir) {
  return collectFiles(dir).filter((path) => basename(path).startsWith("._")).sort();
}

function assertNoAppleDoubleFiles(dir, description) {
  const files = appleDoubleFiles(dir);
  if (files.length === 0) return;

  console.error(`\n[verify-release] Found AppleDouble sidecar files in ${description}:`);
  for (const path of files) {
    console.error(`- ${path}`);
  }
  process.exit(1);
}

function assertEqual(actual, expected, message) {
  if (actual !== expected) {
    console.error(`\n[verify-release] ${message}`);
    console.error(`  expected: ${expected}`);
    console.error(`  actual:   ${actual}`);
    process.exit(1);
  }
}

function assertExecutable(path, message) {
  assertFile(path, message);
  const mode = statSync(path).mode;
  if ((mode & 0o111) === 0) {
    console.error(`\n[verify-release] ${message}: executable bit missing: ${path}`);
    process.exit(1);
  }
}

function plistValue(plistPath, key) {
  const result = spawnSync("plutil", ["-extract", key, "raw", "-o", "-", plistPath], {
    cwd: root,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    console.error(`\n[verify-release] Failed to read ${key} from ${plistPath}`);
    console.error(result.stderr.trim());
    process.exit(result.status ?? 1);
  }
  return result.stdout.trim();
}

function findSingleArtifact(dir, predicate, description) {
  const matches = collectFiles(dir)
    .filter((path) => !basename(path).startsWith("._"))
    .filter(predicate)
    .sort();
  if (matches.length === 0) {
    console.error(`\n[verify-release] ${description} missing under ${dir}`);
    process.exit(1);
  }
  if (matches.length > 1) {
    console.error(`\n[verify-release] Multiple ${description} artifacts found under ${dir}:`);
    for (const path of matches) {
      console.error(`- ${path}`);
    }
    process.exit(1);
  }
  return matches[0];
}

function resolveBundleDir() {
  const candidates = [];
  if (tauriBuildTarget) {
    candidates.push(join(root, "src-tauri", "target", tauriBuildTarget, "release", "bundle"));
  }
  candidates.push(join(root, "src-tauri", "target", "release", "bundle"));

  const existing = candidates.find((path) => existsSync(path));
  if (existing) {
    return existing;
  }

  console.error("\n[verify-release] Tauri bundle directory missing. Checked:");
  for (const path of candidates) {
    console.error(`- ${path}`);
  }
  process.exit(1);
}

function artifactInfo(path) {
  if (!existsSync(path)) {
    return { path, type: "missing", bytes: null, sha256: null };
  }
  const stat = statSync(path);
  if (stat.isFile()) {
    return {
      path,
      type: "file",
      bytes: stat.size,
      sha256: createHash("sha256").update(readFileSync(path)).digest("hex"),
    };
  }
  if (stat.isDirectory()) {
    const files = collectFiles(path).sort();
    const digest = createHash("sha256");
    let bytes = 0;
    for (const file of files) {
      const fileStat = statSync(file);
      const fileHash = createHash("sha256").update(readFileSync(file)).digest("hex");
      const relativePath = relative(path, file).replaceAll("\\", "/");
      bytes += fileStat.size;
      digest.update(`${relativePath}\0${fileStat.size}\0${fileHash}\n`);
    }
    return {
      path,
      type: "directory",
      bytes,
      fileCount: files.length,
      sha256: digest.digest("hex"),
    };
  }
  return { path, type: "other", bytes: null, sha256: null };
}

function verifyMacAppBundle(appPath) {
  assertNoAppleDoubleFiles(appPath, "macOS app bundle");

  const infoPlistPath = join(appPath, "Contents", "Info.plist");
  assertFile(infoPlistPath, "macOS Info.plist missing");

  const metadata = {
    bundleIdentifier: plistValue(infoPlistPath, "CFBundleIdentifier"),
    displayName: plistValue(infoPlistPath, "CFBundleDisplayName"),
    name: plistValue(infoPlistPath, "CFBundleName"),
    shortVersion: plistValue(infoPlistPath, "CFBundleShortVersionString"),
    bundleVersion: plistValue(infoPlistPath, "CFBundleVersion"),
    executable: plistValue(infoPlistPath, "CFBundleExecutable"),
    category: plistValue(infoPlistPath, "LSApplicationCategoryType"),
    minimumSystemVersion: plistValue(infoPlistPath, "LSMinimumSystemVersion"),
  };

  assertEqual(metadata.bundleIdentifier, tauriConfig.identifier, "macOS bundle identifier mismatch");
  assertEqual(metadata.displayName, tauriConfig.productName, "macOS display name mismatch");
  assertEqual(metadata.name, tauriConfig.productName, "macOS bundle name mismatch");
  assertEqual(metadata.shortVersion, tauriConfig.version, "macOS short version mismatch");
  assertEqual(metadata.bundleVersion, tauriConfig.version, "macOS bundle version mismatch");
  assertEqual(
    metadata.minimumSystemVersion,
    tauriConfig.bundle.macOS.minimumSystemVersion,
    "macOS minimum system version mismatch",
  );

  const executablePath = join(appPath, "Contents", "MacOS", metadata.executable);
  assertExecutable(executablePath, "macOS bundle executable missing");

  return {
    ...artifactInfo(appPath),
    metadata: {
      infoPlistPath,
      executablePath,
      ...metadata,
    },
  };
}

function writeEvidenceReport(status = "passed", failure = null) {
  const evidenceDir = join(root, "docs", "release-evidence");
  mkdirSync(evidenceDir, { recursive: true });
  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  const reportPath = join(evidenceDir, `verify-release-${process.platform}-${timestamp}.json`);
  const report = {
    generatedAt: new Date().toISOString(),
    status,
    platform: platform(),
    platformRelease: release(),
    arch: arch(),
    node: process.version,
    rustc: capture("rustc", ["--version"]),
    cargo: capture("cargo", ["--version"]),
    tauriCli: capture("npm", ["run", "tauri", "--", "--version"]),
    tauriConfig: {
      productName: tauriConfig.productName,
      version: tauriConfig.version,
      identifier: tauriConfig.identifier,
      bundleTargets: tauriConfig.bundle.targets,
      buildTarget: tauriBuildTarget || null,
      bundleDir: resolvedBundleDir ?? null,
      windowsInstallMode: tauriConfig.bundle.windows?.nsis?.installMode ?? null,
      macOSMinimumSystemVersion: tauriConfig.bundle.macOS?.minimumSystemVersion ?? null,
    },
    commands: requiredReleaseCommands,
    commandResults,
    artifacts: verifiedArtifacts,
    failure,
    remainingManualEvidence: [
      "Windows/macOS 双端安装后首次启动",
      "Windows 与 macOS 互为主机/客户端连接",
      "跨平台备份恢复",
      "双端 Excel 导入导出人工打开确认",
    ],
  };
  writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
  console.log(`\n[verify-release] Evidence report: ${reportPath}`);
}

run("npm", ["run", "build"]);
run("npm", ["run", "verify:coverage"]);
run("npm", ["run", "verify:no-placeholders"]);
assertNoAppleDoubleFiles(join(root, "src-tauri", "icons"), "Tauri icon source directory");
run("cargo", ["fmt", "--check"], { cwd: join(root, "src-tauri") });
run("cargo", ["test"], { cwd: join(root, "src-tauri") });
const tauriBuildArgs = ["run", "tauri", "--", "build"];
if (tauriBuildTarget) {
  tauriBuildArgs.push("--target", tauriBuildTarget);
}
run("npm", tauriBuildArgs);

resolvedBundleDir = resolveBundleDir();
if (process.platform === "darwin") {
  const dmgPath = findSingleArtifact(
    join(resolvedBundleDir, "dmg"),
    (path) => path.endsWith(".dmg") && path.includes(`${tauriConfig.productName}_${tauriConfig.version}`),
    "macOS DMG",
  );
  const appPath = join(resolvedBundleDir, "macos", `${tauriConfig.productName}.app`);
  assertFile(appPath, "macOS app bundle missing");
  verifiedArtifacts.push(artifactInfo(dmgPath), verifyMacAppBundle(appPath));
  run("hdiutil", ["verify", dmgPath]);
} else if (process.platform === "win32") {
  const files = collectFiles(resolvedBundleDir);
  const installers = files.filter((path) => path.endsWith(".exe") || path.endsWith(".msi"));
  if (installers.length === 0) {
    console.error(`\n[verify-release] Windows installer missing: expected .exe or .msi under ${resolvedBundleDir}`);
    process.exit(1);
  }
  console.log("\n[verify-release] Windows installer artifacts:");
  for (const path of installers) {
    console.log(`- ${path}`);
    verifiedArtifacts.push(artifactInfo(path));
  }
} else {
  console.log(`\n[verify-release] Build completed on ${process.platform}; installer artifact checks are only defined for macOS and Windows.`);
}

const strayDirs = collectFiles(join(root, "src-tauri"))
  .filter((path) => /[/\\]src-tauri[/\\][CZ]:/.test(path));
if (strayDirs.length > 0) {
  console.error("\n[verify-release] Found stray Windows-style test artifacts under src-tauri:");
  for (const path of strayDirs) {
    console.error(`- ${path}`);
  }
  process.exit(1);
}

writeEvidenceReport();
console.log("\n[verify-release] Release verification completed.");
