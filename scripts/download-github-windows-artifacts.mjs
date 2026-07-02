import { createWriteStream, existsSync, mkdirSync, rmSync, statSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";

const root = process.cwd();
const ownerRepo =
  process.env.GITHUB_REPOSITORY?.trim() ||
  process.env.ASTER_GITHUB_REPOSITORY?.trim() ||
  "whalesky-labs/Aster";
const workflowFile = process.env.ASTER_GITHUB_WORKFLOW?.trim() || "build-desktop.yml";
const branch = process.env.ASTER_GITHUB_BRANCH?.trim() || "main";
const apiBaseUrl = process.env.ASTER_GITHUB_API_BASE_URL?.trim() || "https://api.github.com";
const token = process.env.GITHUB_TOKEN?.trim() || process.env.GH_TOKEN?.trim() || "";
const outputArg = process.argv[2] ?? join("docs", "manual-acceptance", "downloaded-windows-artifacts");
const outputDir = resolve(root, outputArg);
const artifactNames = ["aster-windows-x64", "aster-windows-x64-release-evidence"];

if (!token) {
  console.error("[download-github-windows-artifacts] Missing GITHUB_TOKEN or GH_TOKEN.");
  console.error("Create a GitHub token with Actions read access, then run:");
  console.error("  GITHUB_TOKEN=<token> npm run acceptance:download-windows-artifacts");
  process.exit(1);
}

function apiUrl(path) {
  return `${apiBaseUrl.replace(/\/$/, "")}/repos/${ownerRepo}${path}`;
}

async function requestJson(path) {
  const response = await fetch(apiUrl(path), {
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "User-Agent": "aster-acceptance-artifact-downloader",
      "X-GitHub-Api-Version": "2022-11-28",
    },
  });
  if (!response.ok) {
    const body = await response.text();
    throw new Error(`${response.status} ${response.statusText} for ${path}\n${body}`);
  }
  return response.json();
}

async function downloadZip(artifact) {
  const zipPath = join(outputDir, `${artifact.name}.zip`);
  const response = await fetch(artifact.archive_download_url, {
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "User-Agent": "aster-acceptance-artifact-downloader",
      "X-GitHub-Api-Version": "2022-11-28",
    },
    redirect: "follow",
  });
  if (!response.ok || !response.body) {
    const body = await response.text();
    throw new Error(`Failed to download ${artifact.name}: ${response.status} ${response.statusText}\n${body}`);
  }
  await pipeline(Readable.fromWeb(response.body), createWriteStream(zipPath));
  return zipPath;
}

function unzip(zipPath, targetDir) {
  mkdirSync(targetDir, { recursive: true });
  const result = spawnSync("unzip", ["-q", "-o", zipPath, "-d", targetDir], {
    cwd: root,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(`unzip failed for ${zipPath}\n${result.stdout}\n${result.stderr}`);
  }
}

function runImport() {
  const result = spawnSync("npm", ["run", "acceptance:import-windows-artifacts", "--", outputDir], {
    cwd: root,
    env: process.env,
    shell: process.platform === "win32",
    stdio: "inherit",
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

const runs = await requestJson(`/actions/workflows/${workflowFile}/runs?branch=${encodeURIComponent(branch)}&status=success&per_page=20`);
const run = (runs.workflow_runs ?? []).find((item) => item.conclusion === "success");
if (!run) {
  console.error(`[download-github-windows-artifacts] No successful ${workflowFile} run found on ${branch}.`);
  process.exit(1);
}

const artifacts = await requestJson(`/actions/runs/${run.id}/artifacts?per_page=100`);
const selected = artifactNames.map((name) => {
  const artifact = (artifacts.artifacts ?? []).find((item) => item.name === name && !item.expired);
  if (!artifact) {
    throw new Error(`Missing artifact ${name} in workflow run ${run.id}.`);
  }
  return artifact;
});

if (existsSync(outputDir)) {
  rmSync(outputDir, { recursive: true, force: true });
}
mkdirSync(outputDir, { recursive: true });

for (const artifact of selected) {
  const zipPath = await downloadZip(artifact);
  const targetDir = join(outputDir, artifact.name);
  unzip(zipPath, targetDir);
  const size = statSync(zipPath).size;
  console.log(`[download-github-windows-artifacts] Downloaded ${basename(zipPath)} (${size} bytes)`);
  console.log(`[download-github-windows-artifacts] Extracted ${artifact.name} to ${targetDir}`);
}

console.log(`[download-github-windows-artifacts] Source run: ${run.html_url}`);
runImport();
