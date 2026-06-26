# Volta-X Fork 实施方案

## 概要

本文档记录 Volta-X fork 需要实现的功能改造范围、命令行为语义、测试要求，以及 GitHub Actions 发布配置要求。它是后续工程实现的方案文档，不替代 README 或用户手册。

本次 fork 的核心目标是：

- 让 `volta install` 只在没有默认版本时设置默认值，避免覆盖用户已有默认工具版本。
- 引入显式的 `volta default` 命令，用于设置默认工具版本。
- 引入目录级版本选择能力，支持 `volta use`、`.nvmrc` 和 `.node-version`。
- 支持卸载特定工具版本，例如 `volta uninstall node@10`。
- 将官方 upstream 的安装与发布来源替换为本 fork 的 GitHub 仓库来源，并通过 GitHub Actions 构建目标产物。

## 命令语义

### `volta install`

`volta install` 继续负责解析、下载并安装工具版本，但默认版本写入逻辑需要调整：

- 如果对应工具尚未设置默认版本，则安装完成后将本次安装版本设置为默认版本。
- 如果对应工具已经设置过默认版本，则只下载和安装本次请求的版本，不覆盖已有默认版本。
- 对于全局包安装，保持现有行为不变，因为全局包安装依赖默认平台镜像。

示例：

```sh
volta install node@18
volta install node@20
```

如果第一次命令已经把 `node@18` 设为默认版本，第二次命令只安装 `node@20`，不把默认版本改成 `node@20`。

### `volta default`

新增 `volta default <tool[@version]>...` 命令，用于显式设置默认工具版本。

该命令的版本解析语义与 `volta install` 保持一致，但有一个关键差异：

- `volta default` 不执行下载。
- 如果解析后的版本在本地不存在，则命令报错，并提示用户先运行 `volta install`。
- 仅当版本已经存在于本地 inventory / image 目录时，才写入默认平台配置。

支持范围：

- `node`
- `npm`
- `pnpm`
- `yarn`
- `npm@bundled`

示例：

```sh
volta default node@18
volta default npm@bundled
volta default yarn@1.22.22
```

### `volta use`

恢复并公开 `volta use` 命令，用于记录“目录 -> 工具版本”的本地映射。

行为要求：

- `volta use` 不修改项目文件。
- 命令在当前目录记录工具版本配置。
- 记录路径需要使用规范化后的绝对路径。
- 目录配置对该目录及其子目录生效。
- 当多个 `volta use` 目录匹配当前路径时，选择最长祖先路径对应的配置。

建议存储位置：

```text
$VOLTA_HOME/tools/user/directory-platforms.json
```

示例：

```sh
cd ~/work/project-a
volta use node@18

cd ~/work/project-a/packages/app
node --version
```

在子目录执行 `node` 时，应继承 `~/work/project-a` 中记录的 `node@18`。

## 自动切换优先级

工具版本选择需要接入现有 shim 执行流程。每次通过 Volta shim 执行 `node`、`npm`、`pnpm`、`yarn` 或全局二进制时，都根据当前工作目录重新计算有效平台。

优先级必须固定为：

```text
volta pin > .nvmrc > .node-version > volta use > 默认版本
```

具体规则：

- `volta pin` 指 `package.json` 中的 `volta` 配置，保持最高优先级。
- `.nvmrc` 只影响 Node 版本。
- `.node-version` 只影响 Node 版本。
- `.nvmrc` 优先于 `.node-version`。
- `volta use` 低于项目文件配置，高于默认版本。
- 当高优先级来源只提供 Node 版本时，`npm`、`pnpm`、`yarn` 可以从低优先级来源继承。

`.nvmrc` 和 `.node-version` 的解析应支持：

- `18.19.0`
- `v18.19.0`
- semver range
- `lts`
- `latest`

## 特定版本卸载

`volta uninstall` 需要支持删除特定工具版本。

示例：

```sh
volta uninstall node@10
volta uninstall npm@8.1.5
volta uninstall pnpm@7.7.1
volta uninstall yarn@1.22.22
```

行为要求：

- 只删除指定工具的指定版本。
- 删除对应 inventory / image 目录及必要元数据。
- 不影响同一工具的其他版本。
- 如果要删除的版本正在被默认配置引用，则拒绝删除。
- 如果要删除的版本正在被当前项目有效平台引用，则拒绝删除。
- 如果要删除的版本被 `volta use` 目录映射引用，则拒绝删除。
- 对已有包卸载行为保持兼容。

## GitHub 仓库与发布来源

项目中从 upstream 官方页面或仓库拉取信息和文件的路径，需要改为本 fork 的 GitHub 仓库来源。

需要调整的典型来源包括：

- `https://volta.sh/latest-version`
- `https://github.com/volta-cli/volta/releases`
- README 中的官方 badge 和安装说明链接。
- Cargo package metadata 中的 repository 字段。
- Unix installer 和 bootstrap installer 中的 release 下载地址。

推荐实现方式：

- GitHub Actions 中优先使用 `GITHUB_REPOSITORY`。
- 本地或外部脚本中允许使用 `VOLTA_REPO` 覆盖仓库来源。
- 下载地址从当前 fork 的 GitHub Releases 推导，不再依赖 upstream 官方页面。

示例：

```sh
VOLTA_REPO="${VOLTA_REPO:-${GITHUB_REPOSITORY:-owner/volta-x}}"
RELEASE_URL="https://github.com/${VOLTA_REPO}/releases"
```

其中 `owner/volta-x` 应在实现时替换为最终 fork 仓库名，或作为明确 fallback 保留。

## GitHub Actions 配置

需要生成或更新 GitHub Actions workflow，用于测试和发布目标产物。

### 测试 Workflow

建议新增或更新：

```text
.github/workflows/test.yml
```

应覆盖：

- Rust toolchain 安装。
- 格式检查。
- Clippy 检查。
- 单元测试。
- 与本次改造相关的 acceptance / smoke 测试。

### 发布 Workflow

建议新增或更新：

```text
.github/workflows/release.yml
```

应覆盖：

- tag 推送触发发布构建。
- Linux x64 构建。
- Linux ARM 构建。
- macOS universal 构建。
- Windows x64 构建。
- Windows ARM 构建。
- 上传 GitHub Release artifacts。
- artifact 文件名与安装脚本的下载逻辑保持一致。

发布产物应通过本 fork 的 GitHub Actions 构建，不再依赖 upstream 产物。

## 测试计划

### `volta install`

- 首次安装某工具版本时，如果没有默认版本，应设置默认版本。
- 已有默认版本后，再安装其他版本，不应覆盖默认版本。
- 全局包安装行为保持兼容。

### `volta default`

- 本地存在目标版本时，可以设置默认版本。
- 本地不存在目标版本时，命令失败。
- 命令失败时不触发下载。
- `npm@bundled` 行为与现有默认 npm 语义一致。

### `volta use`

- 当前目录记录版本后，在子目录执行 shim 能自动继承。
- 多个祖先目录都存在映射时，最长匹配路径生效。
- 不修改项目内文件。

### 自动切换优先级

- `package.json` 中的 `volta` 配置优先于 `.nvmrc`。
- `.nvmrc` 优先于 `.node-version`。
- `.node-version` 优先于 `volta use`。
- `volta use` 优先于默认版本。
- `.nvmrc` 和 `.node-version` 只影响 Node，其他工具可从低优先级来源继承。

### 特定版本卸载

- 可以删除未被引用的特定工具版本。
- 删除特定版本不影响其他版本。
- 默认版本被引用时拒绝删除。
- 当前项目有效版本被引用时拒绝删除。
- `volta use` 映射引用时拒绝删除。
- 原有包卸载测试继续通过。

### GitHub Actions 与发布脚本

- workflow YAML 语法有效。
- release artifact 名称与 installer 下载逻辑一致。
- installer 能通过 `VOLTA_REPO` 或 `GITHUB_REPOSITORY` 推导下载地址。
- 不再从 upstream `volta.sh` 或 `volta-cli/volta` 拉取 fork 自身发布产物。

## 边界与默认假设

- “进入目录时自动切换”通过 Volta shim 在命令执行时根据当前工作目录选择版本实现，不引入 shell `cd` hook 或后台守护进程。
- `.nvmrc` 和 `.node-version` 是 Node-only 配置来源。
- `volta use` 是机器本地配置，不写入项目文件，也不适合作为团队共享配置。
- `volta pin` 继续作为项目共享配置的最高优先级来源。
- `volta default` 是显式改变默认版本的唯一推荐命令。
- GitHub 仓库 fallback 名称需要在实现阶段替换为最终 fork 的真实 `owner/repo`。
