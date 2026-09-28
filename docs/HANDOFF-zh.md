# 交接说明（中文）

本文件记录工作副本里**尚未提交**的改动、验证到什么程度、以及明确的待办。格式与规则的
权威说明是 `SPEC.md`，设计取舍的编号索引是 `docs/DECISIONS.md`（D1–D19），功能优先级
表是 `docs/FEATURES.md`。中文操作教程在 `docs/USAGE-zh.md`（第八、九节是本批新加的）。

## 1. 当前状态

```
cargo test                15 个测试目标全部通过
sop validate              0 error(s), 0 warning(s)
ui/dist                   已重新构建（History 按钮、Edit 面板、Run 面板、sensor/hardware/
                          conditions 输入、快捷键都在里面）
git                       全部未提交，工作副本干净可继续
```

改动规模：约 40 个文件（含 `ui/dist` 重新构建的产物与删除的 `tools/`）。准确数字用
`git diff --stat` 自己看；本文件的数字不追着维护。

**验证到哪一步**：Rust 侧全部有测试且通过；数据层链路（解析 → 解析后的 step → 事件
日志 → 记录文件）在临时仓库里端到端跑通过。**前端三处改动只有 `npm run build` 通过、
类型对齐，没有在运行中的窗口里点过**——接手后第一件事建议 `./run-app.sh`（或
`REBUILD_UI=1 ./run-app.sh`）实际点一遍 Run / History / Edit 三个视图。

## 2. 未提交改动（按主题）

### 主题 1：运行记录只按自己的版本解释（D15）

- 之前 `sop run end` 之后校验记录时，`step` id 是拿**当前** checklist 去解析的，违反
  `SPEC.md` §8。后果很具体：在 app 里取消一个 include，所有引用过该 procedure 步骤的
  记录就报错，编辑被回滚，提示"这个改动破坏了仓库里的其他东西"。
- 现在 `Repo::run_snapshot_step_ids` 读 `runs/<sop>/<run_id>/snapshot.md`；
  `check::Citation` 分 `Strict` / `Advisory`：没有 snapshot 的旧记录，只有"引用了不存在的
  step"和"complete 覆盖不全"降级为 warning，其余（deviated 缺 reason、
  `deviations_count` 不符）仍是 error。
- 文件：`crates/sop-repo/src/lib.rs`、`crates/sop-core/src/check.rs`、
  `crates/sop-repo/src/validate.rs`、`SPEC.md` §4/§8/§13。

### 主题 2：记录的渲染与导出、删除（D16）

- **真 bug**：`render_record` 之前把模板的 `- [ ]` 原样打出来，一次全勾完的 run 和一次
  没勾的 run 记录长得一样。现在 `body_with_checkbox_state` 按事件日志还原 `- [x]`。
- `sop run export <sop> --format markdown --run <id> [--out PATH]`：打印/写出
  `sop run end` 提交的那份文档；`--format csv` 不变。
- `sop run delete <sop> <run_id> [--yes]`：删且只删三处（记录文件、run 目录、log 目录），
  不带 `--yes` 只列出。
- `authoring::remove_range` 去掉最后一项会留下多余空行 → 校验器判 "file ends with more
  than one newline" → 编辑被回滚。已修，并补了回归测试。
- 另外删掉了仓库里残留的临时 runs（`runs/*/test*.md` 等未跟踪文件），它们曾让整个仓库
  校验失败，从而使**任何**编辑都被回滚。
- 文件：`crates/sop-core/src/run.rs`、`crates/sop-repo/src/run.rs`、
  `crates/sop-cli/src/main.rs`、`crates/sop-core/src/authoring.rs`。

### 主题 3：app 里的"导出记录 / 推送仓库"（D17）

- History 视图按 **test plan / test case** 分组（run 记录的 `sop` 与用例 front matter 的
  `sop_id` 对应，匹配不到用例的 run 归入 "Runs outside a test plan"），每行一个
  **Export record**（写 `exports/<sop_id>-<run_id>.md`，`exports/` 已进 `.gitignore`，
  因为记录本身已在 `runs/` 里），工具栏一个 **Export summary**（按 plan/case 顺序输出
  一个 markdown：先是对每个用例的覆盖行——没跑过的用例也在，run 数为 0——再是每个 run
  的明细行）和一个 **Push repo**（`git add -A` → commit → `push <remote> HEAD`）。
- app 不持有凭据，调用 git 时带 `GIT_TERMINAL_PROMPT=0`，缺凭据是报错而不是弹窗等待。
  commit 成功而 push 失败时 commit 保留。报告逐行回显在页面上。
- 对没有 run 目录的旧记录，`run::record_document` 直接返回 `runs/<sop>/<run_id>.md`
  本身（`SPEC.md` §8 明确允许这种记录存在），所以导出不会对最需要导出的记录失败。
- 文件：`crates/sop-repo/src/git.rs`（`commit_and_push` + 4 个测试，含一个真 git remote）、
  `crates/sop-app/src/commands.rs`（`run_export` / `repo_push`）、
  `ui/src/components/HistoryView.svelte`、`ui/src/lib/api.ts`、`ui/src/lib/types.ts`。
- **注意**：这是 app 唯一会触网的操作，`docs/FEATURES.md` 里两条"永不联网"的条目已按
  事实改掉。

### 主题 4：一份 md → steps 的规则被写清楚（D18）

- `SPEC.md` §6 原来写 "`##` heading followed by **optional** structured metadata"，
  实现却要求必须有 `yaml step` 块。现在 §6/§13 明确：**没有块的章节是散文**。
- 校验器不再只说 `no steps defined`，而是逐章点名：
  `chapter 'First chapter' has no 'yaml step' block, so it is not a step; add one with an 'id'`。
- `id: 7`（YAML 裸数字）原来报 "step has no 'id'"，现在报
  `step 'id' must be a string; write an all-digit id in quotes`。
- 不改"从标题自动生成 id"的原因：id 是记录引用键（§4），标题是显示文本；按位置/标题
  生成的 id 会在插入章节或改标题时改变，`sop run deviations` 这类跨 run 统计会把同一个
  步骤拆成两个桶。
- 文件：`crates/sop-core/src/check.rs`、`crates/sop-repo/src/validate.rs`、
  `crates/sop-repo/tests/negative.rs`、`help/authoring.md`。

### 主题 5：运行视图真的显示你写的内容

- 之前勾选项在 Run 视图里显示成 `Item 1` / `Item 2`，作者写的文字根本不出现。现在
  `RunStepView.checklist` 带条目文字 + 勾选状态（`sop_core::run::checklist_state`），正文
  里的 `- [ ]` 行不再重复渲染（`body_without_checklist`）。
- step 列表与徽章按状态着色（`data-state` / `.state.*`），severity 圆点保留为第二个维度。
- 步骤状态颜色相关改动在 `ui/src/components/ExecutionView.svelte`、`ui/src/app.css`。

### 主题 6：Edit 面板一次只编辑一个 step

- 左列表（编号 + 标题 + id/来源）+ 右表单；`Procedures to include` 和 `Add a step` 折进
  `<details>`，页面不用滚 17 个复选框。
- **重要行为**：列表里是整份解析后的清单，包含 include 进来的步骤。编辑会写回**定义该
  步骤的那个文件**（procedure），不是清单；只有清单自己定义的步骤才显示上移/下移/删除。
  `crates/sop-repo/tests/authoring.rs` 里有一个测试钉住这个语义（对清单路径改 include 进来
  的步骤必须失败）。
- 文件：`ui/src/components/AuthoringPanel.svelte`。

### 主题 7：中文教程与文档

- `docs/USAGE-zh.md` 新增第八节：三条规则表、procedure/checklist 完整示例、app 里三处
  界面分别长什么样、记录里 note/勾选/结果块的真实输出、常见报错表、以及"为什么不能用
  1、2、3 当 id"（`id: "1"` 在 include 时必然撞车，有实测报错）。
- `README.md`、`help/commands.md`、`help/first-run.md`、`help/app-basics.md`、
  `help/authoring.md` 同步了导出/推送/编辑语义。

### 主题 8：仪器 / 条件进记录，结果块可以没有 status（D19）

- **新增三个可选 front matter 块**：`sensor`（`model` / `serial` / `firmware`）、
  `hardware`（列表）、`conditions`（自由 key/value）。Start run 面板里录入，写在
  `RunStarted` 事件的可选字段里（旧日志缺字段照常 replay，新日志在旧构建里也能读，
  所以是 additive，不升 schema）。记录正文开头会**复述**一遍，方便只看导出文件的人。
- **`yaml result` 块可以省略 `status`**，含义是"做了、有数据、没下结论"。有 capture /
  reason 就一定会有块；`complete` 的 run 里不允许出现没有 status 的结果（校验器
  `complete_run_coverage` 会报 `records no outcome`，无 snapshot 时降为 warning）。
- 顺带修掉两处渲染问题：正文里 include marker 不再泄漏进 snapshot / 界面 / 记录
  （`document::prose_of` 按 step span 过滤），以及有勾选项的步骤后面多一个空行
  （现在每步之间恰好一个空行，有回归测试）。
- **键盘快捷键补上了**（`help/app-basics.md` 是 UX 契约）：`space` / `s` / `d` / `n` /
  `j` / `k` / `ctrl+enter` 在 Run 视图生效，capture 输入框里的 `enter` 会跳到下一个
  输入框、最后一个则把这一步标记 done；同时在输入框里时只有 `ctrl+enter` 生效。
  `App.svelte` 原本全局绑的 `j`/`k` 现在只作用于 browse 视图，避免两处同时翻页。
- **删除了过渡实现 `tools/`（4 个 py 文件）和 `docs/PARITY.md`（D11 的执行）**，
  并移除了 CI 里的 `python-cross-check` job；`SPEC.md`、`README.md`、`docs/DESIGN.md`
  §9、`crates/*/src` 里指向 `tools/validate.py` 的注释都改成了"Rust 实现即唯一实现"。
- 文件：`crates/sop-core/src/run.rs`、`crates/sop-core/src/check.rs`、
  `crates/sop-repo/src/run.rs`、`crates/sop-app/src/{api,commands}.rs`、
  `ui/src/components/ExecutionView.svelte`、`ui/src/lib/{api,types}.ts`、
  `SPEC.md` §4/§8/§13、`SPEC-COMPAT.md`、`docs/DECISIONS.md` D19、
  `docs/FEATURES.md`、`docs/USAGE-zh.md` §9、`.github/workflows/validate.yml`。

## 3. 待办 / 已知问题

已解决（本批完成，留作记录）：未完成步骤的 captures 会丢 → D19 允许结果块省略 `status`；
help 承诺的快捷键没实现 → 已在 Run 视图实现并按视图划分；`tools/` + `docs/PARITY.md`
删除 → D11 已执行。

1. **前端改动没有在真实窗口里点过**。`npm run check`（0 error）与 `npm run build` 都通过，
   Rust 侧测试全绿，但 Run 视图的新输入框（sensor/hardware/conditions）、新记录渲染和
   快捷键是纯静态验证。接手后建议 `REBUILD_UI=1 ./run-app.sh`，走一遍：填三项元数据 →
   Start → 只填 capture 不点 Mark done → 结束 → 看 History 的导出结果里有没有数据和空行。
2. **`cargo fmt --all --check` 不通过**（先于本轮存在）。当前 dirty 的文件：

   ```
   crates/sop-app/src/commands.rs      crates/sop-repo/src/git.rs
   crates/sop-cli/src/main.rs          crates/sop-repo/src/lib.rs
   crates/sop-core/src/lib.rs          crates/sop-repo/src/run.rs
   crates/sop-core/src/run.rs          crates/sop-repo/tests/run.rs
   crates/sop-core/src/timestamp.rs
   ```

   （`crates/sop-app/src/main.rs` 只是因为 rustfmt 会跟着 `mod commands;` 走进
   `commands.rs` 才被列出来。）成因是这些文件的行宽风格与 rustfmt 默认值不同，**不要
   整体 reformat**：那会把本轮改动埋进几屏无关的 diff 里。本轮新增/改动的代码里，只有
   `crates/sop-core/src/check.rs` 曾经 dirty，已单独格式化；`crates/sop-core/tests/format.rs`、
   `crates/sop-repo/tests/git.rs`、`crates/sop-repo/tests/negative.rs`、
   `crates/sop-repo/tests/authoring.rs`、`crates/sop-repo/src/validate.rs`、
   `crates/sop-app/src/api.rs` 都是 fmt-clean 的。想在 CI 里加 `cargo fmt --check`，
   得先单独做一次纯格式化提交。
3. `checklists/ground-walk-survey.md` 末尾多了一行
   `<!-- include: procedures/absolute-value-check.md -->`，这是之前用 app 试验 include
  时留下的**内容改动**，不是本轮代码改动；要保留就提交，不要就删掉。

## 4. 怎么跑

```bash
export PATH="$HOME/.cargo/bin:$PATH"

cargo test                                   # 15 个目标
cargo run -q -p sop-cli -- validate          # 0 error(s), 0 warning(s)
REBUILD_UI=1 ./run-app.sh                    # 重建前端并起窗口（隔离的 XDG 目录）

cd ui && npm run build                       # 只重建前端（ui/dist 是被 git 跟踪的）
```

app 的窗口加载 `ui/dist`；改了 `ui/src/**` 必须重新 build，否则看到的是旧界面。
`tauri.conf.json` 的 `frontendDist` 指向 `../../ui/dist`。

## 5. 本轮用过的临时仓库

都是 `/tmp` 下的完整副本，可以直接用来复现，也可以随时删：

- `/tmp/tut-*`：中文教程第八节的示例仓库（新增 `procedures/zero-drift-check.md` 与
  `checklists/drift-survey.md`，含两次 run：`2026-09-26-drift-01`（无结论的步骤）和
  `-02`（备注保留的对照））。路径记在 `/tmp/tut-path`。
- `/tmp/e2e-*`：带真 git remote（`/tmp/e2e-*-origin.git`）的副本，用来验证
  `commit_and_push` 和导出的默认路径。
- `/tmp/vt`：上一轮留下的、带真实 snapshot 的 run 副本。

## 6. 关键文件地图

| 位置 | 是什么 |
|---|---|
| `crates/sop-core/` | 格式：解析、校验、事件状态机、记录渲染（纯函数，无 IO） |
| `crates/sop-repo/` | 文件系统侧：发现、加载、include 解析、原子写、git、run 生命周期 |
| `crates/sop-cli/` | `sop` 命令 |
| `crates/sop-app/` | Tauri 壳：`commands.rs` 是前端能调的全部操作，`api.rs` 是线格式 |
| `ui/src/lib/api.ts` | 前端唯一的 IPC 出口，每个函数对应一个 Tauri 命令 |
| `ui/src/components/AuthoringPanel.svelte` | Edit 视图（一次一个 step） |
| `ui/src/components/ExecutionView.svelte` | Run 视图（状态颜色、勾选项、note） |
| `ui/src/components/HistoryView.svelte` | History 视图（按 plan/case 分组；导出 / 汇总 / 推送 / 删除单条与全部 run） |
| `SPEC.md` | 格式权威说明 |
| `docs/DECISIONS.md` | 设计取舍，D15–D19 是本批改动 |
| `docs/FEATURES.md` | 功能优先级表（本轮待办已登记在此） |
