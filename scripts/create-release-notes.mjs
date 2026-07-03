import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

const [versionArg, tagArg, uploadDirArg = "release-upload", outputArg = "release-notes.md"] = process.argv.slice(2);

const version = versionArg?.replace(/^v/, "") ?? "";
const tag = tagArg || `v${version}`;
const uploadDir = uploadDirArg;
const outputPath = outputArg;
const changelogPath = "CHANGELOG.zh-CN.md";

if (!version) {
  console.error("[create-release-notes] Missing version argument.");
  process.exit(1);
}

if (!existsSync(changelogPath)) {
  console.error(`[create-release-notes] Missing ${changelogPath}.`);
  process.exit(1);
}

const changelog = readFileSync(changelogPath, "utf8");
const exactSection = extractChangelogSection(version);
const unreleasedSection = exactSection ? null : extractChangelogSection("Unreleased");
const sectionSource = exactSection ? version : "Unreleased";
const rawSection = exactSection || unreleasedSection;

if (!rawSection) {
  console.error(`[create-release-notes] Missing changelog section for ${version} and [Unreleased].`);
  process.exit(1);
}

const section =
  sectionSource === "Unreleased"
    ? rawSection.replace(/^## \[Unreleased\]/, `## [${version}] - ${new Date().toISOString().slice(0, 10)}`)
    : rawSection;

const releaseUrl = `https://github.com/${process.env.GITHUB_REPOSITORY ?? "whalesky-labs/Aster"}/releases/download/${tag}`;
const downloads = [
  {
    file: `aster-${version}-macos-x86_64.dmg`,
    usage: "Intel Mac installer",
    purpose: "Intel Mac 安装包",
  },
  {
    file: `aster-${version}-macos-aarch64.dmg`,
    usage: "Apple Silicon Mac installer",
    purpose: "Apple Silicon Mac 安装包",
  },
  {
    file: `aster-${version}-windows-x86_64-setup.exe`,
    usage: "Windows 64-bit installer",
    purpose: "Windows 64 位安装包",
  },
  {
    file: `aster-${version}-windows-x86_64.msi`,
    usage: "Windows 64-bit MSI package",
    purpose: "Windows 64 位 MSI 安装包",
  },
].filter((item) => existsSync(join(uploadDir, item.file)));

if (downloads.length === 0) {
  console.error(`[create-release-notes] No release assets found in ${uploadDir}.`);
  process.exit(1);
}

const downloadRows = downloads
  .map((item) => {
    const url = `${releaseUrl}/${encodeURIComponent(item.file).replaceAll("%2F", "/")}`;
    return `| [\`${item.file}\`](${url}) | ${item.usage} | ${item.purpose} |`;
  })
  .join("\n");

const notes = `${section}

## Downloads / 下载说明

| File / 文件 | Usage | 用途 |
| --- | --- | --- |
${downloadRows}
`;

writeFileSync(outputPath, notes);
console.log(`[create-release-notes] Wrote ${basename(outputPath)} from ${changelogPath}#${sectionSource}.`);

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function extractChangelogSection(name) {
  const sectionPattern = new RegExp(`^## \\[${escapeRegExp(name)}\\](?:\\s|$)`);
  const lines = changelog.split(/\r?\n/);
  const startIndex = lines.findIndex((line) => sectionPattern.test(line));

  if (startIndex === -1) {
    return "";
  }

  const endIndex = lines.findIndex((line, index) => index > startIndex && /^## \[/.test(line));
  return lines.slice(startIndex, endIndex === -1 ? undefined : endIndex).join("\n").trim();
}
