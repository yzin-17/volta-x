# Volta-X Fork 任务 Checklist

本文档根据 [VOLTA_X_FORK_PLAN.md](./VOLTA_X_FORK_PLAN.md) 拆分实施任务。默认按顺序推进；每个阶段完成后应同步运行对应测试。

## 0. 准备与基线确认

- [x] 确认当前分支和工作区状态，避免覆盖未提交改动。
- [x] 跑一次现有核心测试基线，记录当前失败项。
- [x] 梳理命令入口、工具解析、默认平台、项目平台和 shim 执行路径。
- [x] 确认最终 fork 的 GitHub 仓库名 `yzin-17/volta-x`，用于替换 fallback 占位符。

## 1. `volta install` 默认版本行为调整

- [x] 拆分“下载/确保本地存在”和“设置默认版本”的内部逻辑。
- [x] 修改 Node 安装逻辑：已有默认 Node 时不覆盖默认版本。
- [x] 修改 npm 安装逻辑：已有默认 npm 或 bundled 配置时不覆盖默认版本。
- [x] 修改 pnpm 安装逻辑：已有默认 pnpm 时不覆盖默认版本。
- [x] 修改 Yarn 安装逻辑：已有默认 Yarn 时不覆盖默认版本。
- [x] 保持全局 package install 现有行为不变。
- [x] 增加信息日志，说明版本已安装但默认版本未被覆盖。
- [x] 更新或新增 `volta install` acceptance 测试。

## 2. 新增 `volta default`

- [x] 新增 CLI 子命令 `volta default <tool[@version]>...`。
- [x] 新增 `src/command/default.rs` 或等价命令模块。
- [x] 复用 `volta install` 的工具 spec 解析和版本解析语义。
- [x] 增加本地 inventory / image 存在性校验。
- [x] 确保 `volta default` 不触发任何下载或 fetch。
- [x] 本地缺失目标版本时返回清晰错误，并提示先运行 `volta install`。
- [x] 支持 `node`、`npm`、`pnpm`、`yarn`。
- [x] 支持 `npm@bundled`，并保持 bundled npm 语义。
- [x] 更新 help 文案和命令列表。
- [x] 增加 `volta default` acceptance 测试。

## 3. 恢复并实现 `volta use`

- [x] 移除隐藏 deprecated `volta use` 行为。
- [x] 将 `volta use` 显示到 CLI help。
- [x] 定义目录平台配置文件路径：`$VOLTA_HOME/tools/user/directory-platforms.json`。
- [x] 设计目录平台 JSON 结构，支持按绝对路径存储 Node/npm/pnpm/Yarn 配置。
- [x] 执行 `volta use` 时规范化当前目录为绝对路径。
- [x] 支持一次记录多个工具 spec。
- [x] 解析并确保指定版本可用。
- [x] 写入或合并当前目录的版本配置，不修改项目文件。
- [x] 支持 `volta use list` 查看本机目录映射。
- [x] 支持 `volta unuse` 撤销当前目录或 `--dir <path>` 指定目录映射中的指定工具或全部工具。
- [x] 增加目录配置读写单元测试。
- [x] 增加 `volta use` acceptance 测试。

## 4. 自动切换版本来源与优先级

- [x] 新增祖先目录查找逻辑，用于发现 `.nvmrc`。
- [x] 新增祖先目录查找逻辑，用于发现 `.node-version`。
- [x] 新增目录平台映射查找逻辑，按最长祖先路径匹配 `volta use` 配置。
- [x] 接入现有 `Platform::current` / `Session` 平台选择流程。
- [x] 固定优先级：`volta use > volta pin > .nvmrc > .node-version > 默认版本`。
- [x] 确保 `.nvmrc` 只影响 Node。
- [x] 确保 `.node-version` 只影响 Node。
- [x] 支持 `18.19.0`、`v18.19.0`、semver range、`lts`、`latest`。
- [x] 高优先级只提供 Node 时，npm/pnpm/Yarn 从低优先级来源继承。
- [x] 确保 shim 执行时根据当前工作目录重新计算有效平台。
- [x] 增加优先级组合测试。
- [x] 增加继承行为测试。

## 5. 特定版本卸载

- [x] 扩展 `volta uninstall <tool>@<version>` 解析路径。
- [x] 支持卸载特定 Node 版本。
- [x] 支持卸载特定 npm 版本。
- [x] 支持卸载特定 pnpm 版本。
- [x] 支持卸载特定 Yarn 版本。
- [x] 删除对应 image 目录。
- [x] 删除对应 inventory 元数据或缓存文件。
- [x] 删除前检查默认平台引用，命中时拒绝删除。
- [x] 删除前检查当前项目有效平台引用，命中时拒绝删除。
- [x] 删除前检查 `volta use` 目录映射引用，命中时拒绝删除。
- [x] 确保删除一个版本不影响同工具其他版本。
- [x] 保持已有 package uninstall 行为兼容。
- [x] 更新现有“特定版本不支持”测试为新预期。
- [x] 增加版本卸载安全检查测试。

## 6. GitHub 仓库来源改造

- [x] 替换 installer 中的 `https://volta.sh/latest-version`。
- [x] 替换 installer 中的 `https://github.com/volta-cli/volta/releases`。
- [x] 支持通过 `VOLTA_REPO` 指定 fork 仓库。
- [x] GitHub Actions 环境中优先使用 `GITHUB_REPOSITORY`。
- [x] 保留明确 fallback，并在实现时替换为真实 fork `yzin-17/volta-x`。
- [x] 更新 `dev/unix/volta-install.sh`。
- [x] 更新 `dev/unix/volta-install-legacy.sh`。
- [x] 更新 `dev/unix/boot-install.sh`。
- [x] 更新 README badge、安装说明和仓库链接。
- [x] 更新 Cargo metadata repository 字段。
- [x] 检查代码和脚本中残留 upstream 发布来源。

## 7. GitHub Actions 配置

- [x] 生成或更新 `.github/workflows/test.yml`。
- [x] 配置 Rust toolchain 安装。
- [x] 配置格式检查。
- [x] 配置 Clippy 检查。
- [x] 配置单元测试。
- [x] 配置相关 acceptance / smoke 测试。
- [x] 生成或更新 `.github/workflows/release.yml`。
- [x] 配置 tag 推送触发发布构建。
- [x] 配置 Linux x64 构建与 artifact 上传。
- [x] 配置 Linux ARM 构建与 artifact 上传。
- [x] 配置 macOS universal 构建与 artifact 上传。
- [x] 配置 Windows x64 构建与 artifact 上传。
- [x] 配置 Windows ARM 构建与 artifact 上传。
- [x] 确保 release artifact 名称与 installer 下载逻辑一致。
- [x] 确保发布产物来自本 fork 的 GitHub Actions。

## 8. 文档与帮助信息

- [x] 更新 CLI help，包含 `volta default` 和公开的 `volta use`。
- [x] 更新 README 中与 install/default/use/uninstall 相关的用户说明。
- [x] 更新 release / install 文档中关于 fork 仓库来源的说明。
- [x] 确认 `VOLTA_X_FORK_PLAN.md` 与最终实现保持一致。
- [x] 如实现细节变化，同步更新本 checklist。

## 9. 验证与收尾

- [x] 运行格式化检查。
- [x] 运行 `cargo check`。
- [x] 运行单元测试。
- [x] 运行 Clippy。
- [x] 运行单元测试。
- [x] 运行 acceptance 测试。
- [ ] 运行 smoke 测试。（未运行：仓库注释提示会破坏本机 Volta 安装）
- [x] 手工验证 `volta install` 不覆盖默认版本。（acceptance 覆盖）
- [x] 手工验证 `volta default` 不下载且本地缺失时报错。（acceptance 覆盖）
- [x] 手工验证 `volta use` 在子目录生效。（acceptance 覆盖）
- [x] 手工验证 `.nvmrc` / `.node-version` 优先级。（acceptance 覆盖）
- [x] 手工验证特定版本卸载安全检查。（acceptance 覆盖）
- [x] 检查 `rg "volta.sh|volta-cli/volta"` 输出，确认仅保留合理历史引用。
- [x] 检查最终 git diff，确认没有无关改动。
