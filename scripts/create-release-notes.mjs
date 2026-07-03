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
const sectionPattern = new RegExp(
  `(?:^|\\n)(## \\[${escapeRegExp(version)}\\][\\s\\S]*?)(?=\\n## \\[|$)`,
);
const section = changelog.match(sectionPattern)?.[1]?.trim();

if (!section) {
  console.error(`[create-release-notes] Missing changelog section for ${version}.`);
  process.exit(1);
}

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
console.log(`[create-release-notes] Wrote ${basename(outputPath)} from ${changelogPath}#${version}.`);

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
