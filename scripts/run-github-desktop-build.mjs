import { spawnSync } from "node:child_process";

const ownerRepo =
  process.env.GITHUB_REPOSITORY?.trim() ||
  process.env.ASTER_GITHUB_REPOSITORY?.trim() ||
  "whalesky-labs/Aster";
const workflowFile = process.env.ASTER_GITHUB_WORKFLOW?.trim() || "build-desktop.yml";
const token = process.env.GITHUB_TOKEN?.trim() || process.env.GH_TOKEN?.trim() || "";
const apiBaseUrl = process.env.ASTER_GITHUB_API_BASE_URL?.trim() || "https://api.github.com";
const pollIntervalMs = Number(process.env.ASTER_GITHUB_POLL_INTERVAL_MS ?? 30_000);
const timeoutMs = Number(process.env.ASTER_GITHUB_BUILD_TIMEOUT_MS ?? 60 * 60 * 1000);

function currentBranch() {
  const result = spawnSync("git", ["branch", "--show-current"], {
    encoding: "utf8",
  });
  return result.status === 0 ? result.stdout.trim() : "";
}

const branch = process.env.ASTER_GITHUB_BRANCH?.trim() || currentBranch() || "main";

if (!token) {
  console.error("[run-github-desktop-build] Missing GITHUB_TOKEN or GH_TOKEN.");
  console.error("Push the branch first, then run:");
  console.error("  GITHUB_TOKEN=<token> npm run acceptance:run-github-build");
  process.exit(1);
}

if (!Number.isFinite(pollIntervalMs) || pollIntervalMs <= 0) {
  throw new Error("ASTER_GITHUB_POLL_INTERVAL_MS must be a positive number.");
}
if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
  throw new Error("ASTER_GITHUB_BUILD_TIMEOUT_MS must be a positive number.");
}

function apiUrl(path) {
  return `${apiBaseUrl.replace(/\/$/, "")}/repos/${ownerRepo}${path}`;
}

async function request(path, options = {}) {
  const response = await fetch(apiUrl(path), {
    ...options,
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      "User-Agent": "aster-github-desktop-build-runner",
      "X-GitHub-Api-Version": "2022-11-28",
      ...(options.headers ?? {}),
    },
  });
  if (!response.ok) {
    const body = await response.text();
    throw new Error(`${response.status} ${response.statusText} for ${path}\n${body}`);
  }
  if (response.status === 204) return null;
  return response.json();
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function runDownload() {
  const npmCommand = process.env.ASTER_NPM_COMMAND?.trim() || "npm";
  const result = spawnSync(npmCommand, ["run", "acceptance:download-windows-artifacts"], {
    env: {
      ...process.env,
      ASTER_GITHUB_BRANCH: branch,
      ASTER_GITHUB_WORKFLOW: workflowFile,
      ASTER_GITHUB_REPOSITORY: ownerRepo,
      ASTER_GITHUB_API_BASE_URL: apiBaseUrl,
    },
    shell: process.platform === "win32",
    stdio: "inherit",
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

const startedAt = new Date();
await request(`/actions/workflows/${workflowFile}/dispatches`, {
  method: "POST",
  body: JSON.stringify({
    ref: branch,
    inputs: {
      publish_release: "false",
      version_mode: "auto",
      version: "",
    },
  }),
});
console.log(`[run-github-desktop-build] Dispatched ${workflowFile} on ${branch}.`);

const deadline = Date.now() + timeoutMs;
let run = null;
while (Date.now() < deadline) {
  const runs = await request(
    `/actions/workflows/${workflowFile}/runs?branch=${encodeURIComponent(branch)}&event=workflow_dispatch&per_page=20`,
  );
  run = (runs.workflow_runs ?? [])
    .filter((item) => new Date(item.created_at ?? 0).getTime() >= startedAt.getTime() - 60_000)
    .sort((a, b) => new Date(b.created_at ?? 0).getTime() - new Date(a.created_at ?? 0).getTime())[0] ?? null;

  if (run) {
    console.log(`[run-github-desktop-build] Run ${run.id}: ${run.status}/${run.conclusion ?? "pending"}`);
    if (run.status === "completed") break;
  } else {
    console.log("[run-github-desktop-build] Waiting for workflow run to appear...");
  }
  await sleep(pollIntervalMs);
}

if (!run) {
  console.error("[run-github-desktop-build] Timed out waiting for workflow run to appear.");
  process.exit(1);
}
if (run.status !== "completed") {
  console.error(`[run-github-desktop-build] Timed out waiting for run ${run.id} to complete.`);
  process.exit(1);
}
if (run.conclusion !== "success") {
  console.error(`[run-github-desktop-build] Run ${run.id} finished with conclusion: ${run.conclusion}`);
  console.error(run.html_url ?? "");
  process.exit(1);
}

console.log(`[run-github-desktop-build] Run succeeded: ${run.html_url ?? run.id}`);
runDownload();
