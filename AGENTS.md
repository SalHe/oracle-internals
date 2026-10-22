# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## 项目概述

本项目用于逆向分析 Oracle 归档 Redo 文件的物理/逻辑结构。它由两个互补的部分组成：

- **Python 实验台**（仓库根目录 + `oracle_internals/`）：用 Docker 启动 Oracle 19c/21c EE，执行最小化 SQL 用例，归档 redo 日志，并让 Oracle 自己 `ALTER SYSTEM DUMP LOGFILE` 出文本，把 `redo.dbf`（二进制）和 `redo.txt`（Oracle 官方 dump）一起下载到 `out/dumped/` 供对照。
- **Rust 工作区**（`crates/`）：独立实现的 redo 解析器。`ointernal-parser` 是库（结构布局、流式读取、checksum、修复），`ointernal-dumper` 是 CLI（`dump`/`fix`/`represent`）。dump 输出刻意对齐 Oracle 官方 dump 的文本格式，便于逐行 diff。

分析方法的闭环是：Oracle dump ↔ 自行解析 ↔ 修改字节后重新 dump 验证。README 中的 `#{AMND}` / `#{AME}` 标记就是这套验证方法的产物，必须保留。

## 常用命令

### Python 实验台（需要 Docker + Oracle 镜像）

```bash
uv sync                                   # 安装依赖（uv.lock）
./olabs init                              # 启动并初始化 DB 容器，等待 ready-tag 后停止
./olabs uninit                            # 停止并删除容器

# 跑全部/单个用例；`--` 之后的参数原样传给 pytest
./olabs collect -- -k TestInsertIntoOneColumn -s -v
./olabs collect -- -k test_update

# redo 文件操作（默认在容器内 archive_logs 目录）
./olabs redo ls
./olabs redo cp 1_45_1224427833.dbf ./out/
./olabs redo dump 1_45_1224427833.dbf
./olabs redo dump out/dumped/.../redo.dbf -l          # 本地文件，先上传到容器再 dump

# 修改字节 → 自动重算 checksum → 交给 Oracle dump 验证
./olabs redo dump out/.../redo.dbf -l -r '0x400,AABBCCDD' -r '0x404,u32:0x11223344'

# 选择 DB 版本（默认 ./oracle-internal.yaml，CI 用 oracle-internal-ci-*.yaml）
uv run main.py --config oracle-internal-ci-19c-ee.yaml init
```

注意：`--config` 是顶层参数，必须写在子命令之前。不要直接 `pytest`，测试运行时要读取 CLI 启动时填充的全局 `CONFIG`（`oracle_internals/config.py`），否则会 KeyError。

### Rust

```bash
cargo build
cargo test --workspace
cargo test -p ointernal-parser checksum::test::checksum   # 单个测试
cargo fmt
cargo clippy --workspace

./odumper dump out/dumped/oracle_internals.cases.test_insert_into/TestInsertIntoOneColumn/redo.dbf
./odumper dump <redo> --filter 'layer == 11 && op == 2'   # 变量：record/change/layer/op/scn
./odumper dump <redo> --force-vector                     # 未识别 op 也打印原始 vector
RUST_LOG=trace ./odumper dump <redo>                     # tracing 输出到 stderr

./odumper fix <redo> -r '0x400,AABBCCDD' -r '0x404,u16:1234'  # 生成 <redo>.fixed
./odumper represent 'c102'
```

`./odumper` 只是 `cargo run -- "$@"`；需要显式指定 crate 时用 `cargo run -p ointernal-dumper -- ...`。

CI（`.github/workflows/ci.yml`）只跑 Python 的 `init` + `collect` 矩阵（19c/21c），**不跑** `cargo test`，合并前需本地执行。

## 测试用例约定

`oracle_internals/cases/` 下所有用例继承 `CaseBase`，只实现三个钩子：

- `_prepare`：建表/预置数据（在归档窗口之外）
- `_run`：被测 SQL（会被归档窗口包住）
- `_cleanup`：清理（在 dump 之后）

`test_base.py` 的 `test_run` 流程固定为：归档当前日志 → 执行 `_run` → 再次归档 → 通过 `v$archived_log` 集合差取出本次产生的 redo → dump 到 trace 文件 → 把 redo 与 trace 下载到
`out/dumped/<module>/<Class>/`（`redo.dbf`、`redo.txt` 为软链接）。

每个测试通过 `@within_container` 自行启停容器并等待 `ready-tag`，以缩小容器存活时间、避免后台 session 污染 redo；因此 `collect` 在调用 pytest 前会先停容器。容器配置（镜像、名称、密码、ready-tag、输出目录）来自 YAML 并写入全局 `CONFIG`。

## Rust 解析架构

解析流水线（`crates/ointernal-parser/src/oracle/`）：

1. `file.rs`：block 0 的 `FileHeader`（magic `0x7A7B7C7D`，含 `block_size`/`block_count`）。
2. `redo.rs`：block 1 的 `RedoFileHead`；每个块的 `RedoBlockHead`（magic `0x2201`）；记录头 `RedoRecordHeadCommon` + `V1`/`V2`；`Change` 头（layer、op、DBA、SCN、change-list 长度等）。
3. `reader.rs`：`RedoFileReader` 负责跨块流式读取、逐块校验 checksum、按记录的 `VLD.kcr_void` 位选择 V2/V1、按 `len` 判断记录边界；`read_change` 先读固定 `Change` 结构，再读 vector 尺寸表（首尺寸在 `Change` 内，其余各占 `u16`）、可选 2 字节对齐 padding，最后读 vector 数据（按 4 字节 padding），`VectorData.ranges` 记录数据在原始文件中的偏移区间。
4. `change/opcode.rs`：`codes_for_layer!` 宏集中注册 `(layer, op) -> 类型`。这是唯一 opcode 注册表；新增 op 必须改这里。
5. `change/cv.rs`：`ChangeAndVector::parse()` 按注册表分发，并通过 `FromCV` 把 vector 字节零拷贝转换成各 layer 的 `#[repr(C)]` 结构。
6. `checksum.rs`：按 8 字节小端字异或折叠；`offset` 最高位为 1 时计算前先把 magic 置最低位。`fixer.rs` 据此逐块重算，`fix.rs` 先把文件复制为 `.fixed` 再替换字节并修复校验和。

Dump 侧（`crates/ointernal-dumper/src/`）：

- `dump/mod.rs`：遍历记录/change、打印 Oracle 风格的 file header、应用 `--filter`、在无 `Dump` 实现或加密时回退打印原始 vector（含文件偏移）。
- `dump/dump_layer{5,11,13}.rs`：为每个 layer 枚举实现 `Dump`，并在枚举 `match` 中挂接。输出文本是给 Oracle dump 对照用的，格式（缩进、`0x` 宽度、每 25 字节换行、4 字节分组）属于行为的一部分。
- `utils.rs` 的 `Hex` 负责十六进制格式化，`filter.rs` 用 `evalexpr` 求值过滤表达式。

### 新增/扩展一种 redo op 的步骤

1. `opcode.rs`：在对应 layer 块中登记 `Name = code !layers::layerN::Type`。
2. `change/layers/layerN.rs`：定义 zerocopy 布局结构，并实现 `FromCV` 从 vector 数据转换。
3. `dump/dump_layerN.rs`：实现 `Dump` 并在 layer 枚举的 `match` 中加分支。
4. `cases/` 下新增复现用例，`./olabs collect -- -k <用例>` 生成 `redo.dbf`/`redo.txt`。
5. 用 `./odumper dump .../redo.dbf` 与 `redo.txt` 逐段对照；必要时用 `-r` 改字节重 dump 验证（并补 `#{AMND}`/`#{AME}` 注释）。

`docs/InsertMultipleRows-Implementation.md` 是这套流程的完整示例；`docs/notes/` 记录 redo 头、block、QMI 的结构结论和外部参考资料。

## 项目特定约定

- **注释标记**：无法确认含义的字段用 `#{AMND}`（修改后无差异）或 `#{AME}`（修改后报错）标注，并在 README 的缩写表中维护；不要用大段重复说明替代标记。
- **布局即协议**：所有磁盘结构体用 `zerocopy` + `#[repr(C)]`，未知字段保留为 `_unknown_N`/`zeros_N` 占位以维持偏移，不要随意删除或重排。
- **配置全局态**：`CONFIG` 在 `main.py` 解析 `--config` 后通过 `CONFIG.update(...)` 填充；读取配置的代码应假定运行在 CLI 分发之后。
- **容器内路径**：SID 固定 `ORCLINTERNAL`，归档目录 `/opt/oracle/oradata/ORCLINTERNAL/archive_logs/`，连接串 `localhost:1521/ORCLINTERNAL`（`sys` 用户，密码来自配置）。
- **生成物**：`out/`、`target/`、`.venv/` 均被忽略；`out/dumped/` 是分析产物，不要提交。
- **语言风格**：Python 注释/提示与文档以中文为主；Rust 代码与 commit message 使用英文。提交前跑 `cargo fmt` 和 `cargo clippy`。
