#!/usr/bin/env bash
set -euo pipefail

step() {
  printf '\n==> %s\n' "$1"
}

force_arg=""
if [[ "${1:-}" == "--force" ]]; then
  force_arg="--force"
fi

step "安装依赖"
npm install

step "执行完整发布验证"
npm run verify:release

step "生成 macOS 实机验收 JSON 和清单"
npm run acceptance:collect -- macos ${force_arg}

step "生成当前剩余证据汇总"
if ! npm run verify:manual-acceptance -- --strict; then
  echo "严格手工验收尚未通过；已生成 remainingEvidence 汇总，继续生成交接包。"
fi

step "生成并校验验收交接包"
npm run acceptance:package
npm run verify:readiness
npm run acceptance:package
npm run verify:acceptance-package
npm run acceptance:archive

step "下一步"
echo "1. 按 docs/manual-acceptance/manual-acceptance-macos-*-checklist.md 完成实机勾验。"
echo "2. 填写 docs/manual-acceptance/manual-acceptance-macos-YYYY-MM-DD.json。"
echo "3. 保留 docs/acceptance-package/、docs/acceptance-archives/*.zip、docs/release-evidence/*.json、docs/manual-acceptance/*.json 和 evidence-* 附件目录。"
echo "4. 在合并后的项目根目录执行 npm run verify:manual-acceptance -- --strict。"
