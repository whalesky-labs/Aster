import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { arch, platform, release } from "node:os";

const root = process.cwd();
const evidenceDir = join(root, "docs", "release-evidence");
const commandResults = [];
const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const reportPath = join(evidenceDir, `verify-all-local-${process.platform}-${timestamp}.json`);

const commands = [
  {
    command: "npm",
    args: ["run", "build"],
    text: "npm run build",
  },
  {
    command: "npm",
    args: ["run", "test:manual-acceptance"],
    text: "npm run test:manual-acceptance",
  },
  {
    command: "npm",
    args: ["run", "verify:no-placeholders"],
    text: "npm run verify:no-placeholders",
  },
  {
    command: "npm",
    args: ["run", "verify:coverage"],
    text: "npm run verify:coverage",
  },
  {
    command: "cargo",
    args: ["fmt", "--check"],
    cwd: join(root, "src-tauri"),
    text: "cargo fmt --check",
  },
  {
    command: "cargo",
    args: ["test"],
    cwd: join(root, "src-tauri"),
    text: "cargo test",
  },
  {
    command: "npm",
    args: ["run", "verify:release"],
    text: "npm run verify:release",
  },
  {
    command: "npm",
    args: ["run", "acceptance:package"],
    text: "npm run acceptance:package",
  },
];

const readinessCommands = [
  {
    command: "npm",
    args: ["run", "verify:readiness"],
    text: "npm run verify:readiness",
  },
];

const finalizationCommands = [
  {
    command: "npm",
    args: ["run", "acceptance:package"],
    text: "npm run acceptance:package",
  },
  {
    command: "npm",
    args: ["run", "verify:acceptance-package"],
    text: "npm run verify:acceptance-package",
  },
  {
    command: "npm",
    args: ["run", "acceptance:archive"],
    text: "npm run acceptance:archive",
  },
];

function writeReport(status, failure = null) {
  mkdirSync(evidenceDir, { recursive: true });
  const report = {
    generatedAt: new Date().toISOString(),
    status,
    platform: platform(),
    platformRelease: release(),
    arch: arch(),
    commands: [...commands, ...readinessCommands, ...finalizationCommands].map((item) => item.text),
    commandResults,
    failure,
    note:
      "本报告证明当前机器可自动执行的本地门禁已完成；Windows/macOS 实机安装、截图、Excel 人工打开、跨平台恢复和互为主机/客户端仍以 strict manual acceptance 为准。",
  };
  writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
  console.log(`\n[verify-all-local] Evidence report: ${reportPath}`);
}

function runCommand(item) {
  const startedAt = new Date();
  console.log(`\n$ ${item.text}`);
  const result = spawnSync(item.command, item.args, {
    cwd: item.cwd ?? root,
    env: process.env,
    shell: process.platform === "win32",
    stdio: "inherit",
  });
  const finishedAt = new Date();
  const commandResult = {
    command: item.text,
    cwd: item.cwd ?? root,
    status: result.status === 0 ? "passed" : "failed",
    exitCode: result.status,
    signal: result.signal ?? null,
    startedAt: startedAt.toISOString(),
    finishedAt: finishedAt.toISOString(),
    durationMs: finishedAt.getTime() - startedAt.getTime(),
  };
  commandResults.push(commandResult);
  if (result.status !== 0) {
    writeReport("failed", {
      failedCommand: commandResult,
    });
    process.exit(result.status ?? 1);
  }
}

for (const item of commands) {
  runCommand(item);
}

writeReport("passed");

for (const item of readinessCommands) {
  runCommand(item);
}

writeReport("passed");

for (const item of finalizationCommands) {
  runCommand(item);
}

writeReport("passed");
console.log("\n[verify-all-local] Local automatic verification completed.");
