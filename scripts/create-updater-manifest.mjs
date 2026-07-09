import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

const [versionArg, tagArg, uploadDirArg = "release-upload", outputArg = "release-upload/latest.json"] =
  process.argv.slice(2);

const version = versionArg?.replace(/^v/, "") ?? "";
const tag = tagArg || `v${version}`;
const uploadDir = uploadDirArg;
const outputPath = outputArg;
const repository = process.env.GITHUB_REPOSITORY ?? "whalesky-labs/Aster";
const releaseUrl = `https://github.com/${repository}/releases/download/${tag}`;

if (!version) {
  console.error("[create-updater-manifest] Missing version argument.");
  process.exit(1);
}

if (!existsSync(uploadDir)) {
  console.error(`[create-updater-manifest] Missing upload directory: ${uploadDir}.`);
  process.exit(1);
}

const assetMap = [
  {
    platform: "darwin-x86_64",
    archive: `aster-${version}-macos-x86_64-updater.tar.gz`,
    signature: `aster-${version}-macos-x86_64-updater.tar.gz.sig`,
  },
  {
    platform: "darwin-aarch64",
    archive: `aster-${version}-macos-aarch64-updater.tar.gz`,
    signature: `aster-${version}-macos-aarch64-updater.tar.gz.sig`,
  },
  {
    platform: "windows-x86_64",
    archive: `aster-${version}-windows-x86_64-setup.exe`,
    signature: `aster-${version}-windows-x86_64-setup.exe.sig`,
  },
];

const missing = assetMap
  .flatMap((item) => [item.archive, item.signature])
  .filter((file) => !existsSync(join(uploadDir, file)));

if (missing.length > 0) {
  console.error("[create-updater-manifest] Missing updater assets:");
  for (const file of missing) {
    console.error(`- ${file}`);
  }
  process.exit(1);
}

const notes = releaseNotes(version);
const platforms = Object.fromEntries(
  assetMap.map((item) => [
    item.platform,
    {
      signature: readFileSync(join(uploadDir, item.signature), "utf8").trim(),
      url: `${releaseUrl}/${encodeURIComponent(item.archive)}`,
    },
  ]),
);

const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms,
};

writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`[create-updater-manifest] Wrote ${basename(outputPath)} for ${version}.`);

function releaseNotes(versionName) {
  const changelogPath = "CHANGELOG.zh-CN.md";
  if (!existsSync(changelogPath)) {
    return `Aster ${versionName}`;
  }
  const changelog = readFileSync(changelogPath, "utf8");
  return (
    extractChangelogSection(changelog, versionName) ||
    extractChangelogSection(changelog, "Unreleased") ||
    `Aster ${versionName}`
  );
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function extractChangelogSection(changelog, name) {
  const sectionPattern = new RegExp(`^## \\[${escapeRegExp(name)}\\](?:\\s|$)`);
  const lines = changelog.split(/\r?\n/);
  const startIndex = lines.findIndex((line) => sectionPattern.test(line));

  if (startIndex === -1) {
    return "";
  }

  const endIndex = lines.findIndex((line, index) => index > startIndex && /^## \[/.test(line));
  return lines
    .slice(startIndex + 1, endIndex === -1 ? undefined : endIndex)
    .join("\n")
    .trim();
}
