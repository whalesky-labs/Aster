# Aster 工程规范

本规范是代码评审和 CI 的硬约束。安全整改设计中的“工程化架构约束”是完整定义，本文件作为日常入口。

## 核心原则

- 前端依赖方向：`app -> features -> entities -> shared`。
- Rust 使用端口与适配器：transport 调用 application，application 依赖 domain 与 ports，infrastructure 实现 ports，composition root 负责装配。
- 公共能力必须语义稳定且至少有两个真实调用方；测试替身所需的时钟、随机源、凭据库和会话存储可在首个实现时定义窄接口。
- 禁止万能 `utils`、`CommonService`、跨层 SQL、隐藏事务和复制安全协议。
- 新文件不超过 500 行；遗留热点只允许缩小，不允许增长。
- 新增 Rust 生产代码禁止 `unwrap`、`expect` 和 `panic!`；遗留使用量只允许下降。
- 新行为必须有成功与失败路径测试，安全缺陷必须有回归测试。

## 本地检查

```bash
npm run verify:engineering
npm run build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

基线位于 `config/engineering-baseline.json`。不得通过扩大基线规避整改；确需豁免时必须在代码评审中说明原因和清理条件。
