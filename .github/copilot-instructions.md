# Copilot / AI Agent 指引（精简版）

目的：帮助 AI 编码代理在此仓库中快速上手。

- **整体概览**：这是一个分析 Oracle redo 文件的混合型 Python + Rust 工作区。
  - Python 的 CLI 和工具库位于仓库根目录和 `oracle_internals/` 目录下（包含测试、CLI、Docker 辅助程序）。
  - Rust 工具位于 `crates/` 目录下（特别是 `ointernal-dumper` 和 `ointernal-parser`）。

- **关键文件与组件**（修改行为时用作参考点）：
  - `main.py` — Python CLI 入口（将 `oracle-internal.yaml` 加载到全局 `CONFIG` 中）。
  - `olabs` — 根目录便捷脚本，等同于 `uv run main.py`，用于快速调用 Python CLI。
  - `odumper` — 根目录便捷脚本，等同于 `cargo run`，用于快速运行 Rust dumper。
  - `oracle_internals/cli/_main.py` — CLI 子命令的注册：`init`、`uninit`、`collect`、`redo`。
  - `oracle_internals/cli/container.py` — Docker 容器生命周期管理（`init`/`remove`）。
  - `oracle_internals/cli/testing.py` — `collect` 命令使用 `pytest`（调用 `pytest.main`），运行测试前会停止容器。
  - `oracle_internals/db.py` — 与 Docker 容器和 Oracle 数据库交互的辅助函数（文件下载、连接管理）。
  - `oracle_internals/cases/` — pytest 测试用例和 fixtures（测试与 Docker DB 容器交互）。
  - `crates/ointernal-dumper` — Rust 二进制程序，用于 dump/修复 redo 文件（`cargo run -p ointernal-dumper -- <cmd>`）。

- **开发工作流 / 命令示例**：
  - 启动容器（使用 yaml 配置）：`./olabs init`
  - 停止/删除容器：`./olabs uninit`
  - 通过 CLI 运行测试：`./olabs collect -- -k <pytest_expr>`（参数传给 pytest）。
  - 直接运行 pytest：`pytest --pyargs oracle_internals.cases -k <expr>`
  - 运行 Rust dumper：`./odumper dump path/to/redo`

- **配置**：
  - 默认配置文件：`./oracle-internal.yaml`（CLI 的 `--config` 参数可覆盖）。CLI 启动时将其加载到 `oracle_internals/config.py` 中的全局 `CONFIG` 字典。
  - Docker 容器设置（镜像、名称、凭证）在 YAML 中定义，由 `container.py` 和 `db.py` 使用。

- **项目特定的约定和注意事项**（必须保持）：
  - 测试位于 `oracle_internals/cases` 目录下，CLI 的 `collect` 命令会在运行 pytest 前停止 DB 容器（参见 `testing.collect_case`）。不要假设测试运行时容器仍在运行。
  - 全局可变配置：代码期望 `CONFIG` 在 CLI 启动时被填充（`main.py` 读取 YAML 并更新 `CONFIG`）。编写读取配置的代码时，要确保运行在 CLI 解析之后，或对缺失的 key 进行守卫检查。
  - Python <-> Docker 集成：`db.py` 使用 Docker API 从容器中获取文件（tar 归档提取）。修改文件传输逻辑时，保持流式和 tar 逻辑不变。
  - Rust 工作区：`ointernal-dumper` 通过 Cargo.toml 中的路径依赖依赖 `ointernal-parser` crate；对跨 crate 的更改优先使用工作区构建。

- **测试与 CI**：
  - 测试基于 pytest；运行 `pytest oracle_internals/cases` 或通过 CLI 的 `collect` 命令执行。
  - GitHub Actions 工作流位于 `.github/workflows/ci.yml`，遵循其步骤以符合 CI 预期。

- **常见工作流示例：分析新的 redo 类型（如 DELETE）**：
  1. 在 `oracle_internals/cases/` 下创建新的测试用例（参考 `test_delete.py`），继承 `CaseBase`，实现 `_prepare`（创建表）、`_run`（执行 DELETE）、`_cleanup`（清理）。
  2. 运行 `./olabs init` 启动 Docker 容器初始化数据库。
  3. 执行测试：`./olabs collect -- -k <test_name>`，该命令会自动生成 redo 文件、执行 Oracle dump，并将输出保存到 `CONFIG['outputs']['dumped']`。
  4. 获取 redo 文件：查看测试输出目录中的 redo 文件和 dump 文本。
  5. 使用 `./odumper dump /path/to/redo` 用自定义的 Rust 解析器分析 redo 文件。
  6. 如要修改 redo 内容验证：使用 `./odumper fix /path/to/redo -r "0x400,AABBCCDD"` 修改指定位置的字节并重新计算 checksum，然后用 Oracle dump 验证。

- **查找实现特性的示例**：
  - CLI 命令注册在 `oracle_internals/cli/_main.py`，处理程序在 `oracle_internals/cli/` 目录下。
  - 容器交互示例在 `oracle_internals/db.py`（get/archive/download 函数）。
  - Rust dump/fix 实现在 `crates/ointernal-dumper/src/`（参见 `main.rs`、`dump/`、`fix.rs`）。

如有任何不清楚的地方，或需要更详细的信息（测试运行示例、配置样本或常见变更模式），请告诉我你需要补充的领域。  
