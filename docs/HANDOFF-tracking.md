# Handoff — 记录对齐 / 跟踪价值（下一会话从这里接起）

> 本会话按「建议的落地顺序」实现了 base + 1 + 2 + 6 + 3 + 5(events 封存) + timeline 导出，并把 4/7/8 与杂项记录在案。
> 写入时间：2026-09-29。所有 Rust 测试通过（18 suites, 0 fail），`svelte-check` 0 error，UI build 通过。已 push 到 origin/main。

## 已完成（本会话）

- **base：事件时间统一由 Rust 打**（4.1 相关）。
  - `RunEvent::stamp(at)` 新增；`sop-repo::run::record_events` 在 append 前用 Rust 时钟覆盖事件 `at`，前端发的 stale 时间被忽略。
  - 前端 `ExecutionView` 在步骤成为当前步骤时发 `StepOpened`（`lastOpenedStep` 守卫避免重复）。
  - 测试：`a_recorded_event_is_stamped_with_the_tools_clock_not_the_senders`。

- **1. 步骤时间窗 + 记录时间线**。
  - `StepState` 增 `opened_at`/`ended_at`；`StepStatusChanged`（done/skip/deviate）写 ended_at，reopen 清空；`StepOpened` 写 opened_at（最后打开者胜）。
  - `render_record` 输出 `## Timeline` 表（步骤/标题/开始/结束/状态/时长），并把 `opened_at`/`ended_at` 写进每个 `yaml result` 块；`timeline_csv()` 输出同表 CSV。
  - 校验器：`missing_step_opened`（警告）、`time_regression`（ended<opened 报错）。
  - 测试：core 4 个 + repo `a_closed_step_renders_its_window_in_the_record_and_csv`。
  - timeline.csv 导出已接到 CLI：`sop run timeline <sop> <run_id>`。

- **2. 附加日志读取时间范围 + 重叠校验**。
  - `AttachmentAdded`/`Attachment` 增可选 `t_min`/`t_max`/`row_count`；`sop-repo::attach` 扫描 CSV 时间列（`timestamp*`/`time*`/`utc*`/`gps_time*`），解析 ISO-8601 或 epoch，失败则省略字段（不报错，符合 D6）。
  - 记录 `logs:` 段写出时间范围/行数，并新增 `step:`（当挂在步骤下）以支持步骤级重叠校验。
  - 校验器：`log_overlap`（日志时间与所属步骤或 run 窗口无交集 → 警告）。
  - 测试：`attaching_a_csv_with_a_time_column_records_its_range`、`attaching_a_file_with_no_time_column_omits_the_range`。

- **3. 时钟偏差和现场标记**。
  - `ClockInfo`（basis / instrument_time / offset_secs），run 开始时记录仪器时钟（`RunStarted.clock`，前端 start 表单输入 + 计算 offset）。
  - `FieldMarker` 事件（工具打时间戳的带标签时刻）；`run_marker` 命令 + 前端 `m` 热键 + 标记列表显示。
  - 记录前件 `clock:` / `markers:` + 正文 `## Field markers` 节。
  - 测试：`a_run_keeps_its_clock_and_dropped_markers`。

- **5. 时间戳可靠性（部分）**。
  - 事件时间倒退校验（`time_regression`）已完成。
  - run 结束时把 `events.jsonl` 的 sha256 封存进记录前件 `events_sha256:`；校验器在日志被改后报错（tamper 检测）。
  - 测试：`an_ended_run_seals_its_event_log_hash_into_the_record`。
  - 剩余：可选的 push 时 RFC 3161 时间戳（需联网，离线后补；git 提交时间不可信）。

- **4. 步骤级 `outputs:` 声明**。
  - `yaml step` 块新增可选 `outputs`（文件列表，如 `mag_raw.csv`）；解析进 `Step.outputs`，经 `ResolvedStepDef` 暴露到执行视图。
  - 前端每步显示「Declared outputs」：已附着的打 ✓，缺失的标 ⚠ not attached。
  - 校验器 `declared_outputs_present`：`complete` run 缺一个已声明输出即报错（"done" 与 "数据收齐" 一致）。
  - 测试：core 2 个 + SPEC 文档。
  - 剩余：authoring 编辑器可编辑 `outputs` 字段（可后续加）。

- **6. 动态加步骤**。
  - `StepAdded` 事件（id 由 Rust 生成 `adhoc-NNN`，`next_adhoc_id`）；快照冻结不变；`RunState.added_steps` 记录。
  - `sop-repo::load` 重放后用 `merge_added_steps` 把 adhoc 步骤按 `after` 折叠进运行时 `defs`，执行视图与覆盖检查都能看见。
  - 校验器：`run_results` 接受 `adhoc-*` 步骤 id；`RunState::deviations()` 排除 adhoc 步骤（added 不算偏差）。
  - 记录：`## Steps added during the run` 节 + 前件 `added_steps` 计数。
  - 命令：`run_add_step`（tauri）+ 前端「Add step」按钮/内联表单。
  - 测试：`a_step_added_mid_run_appears_in_the_view_and_the_record` + core `a_step_added_mid_run_is_merged_at_replay`。

## 待办（未做 / 下一步）

- **7. 同一用例跨 run 对比**。
  - `sop run export <sop> --format csv` 现含 `date` 与 `serial` 列，可在同一用例的多次 run 间看某个 capture 的变化趋势（校准类）。
  - 剩余：History 视图里加对比表（可直接用同一数据源渲染）。
- **8. 日志文件名自动生成**（附加时按 run 元数据给建议名）。
- **5 的剩余**：可选的 push 时 RFC 3161 时间戳。

## 杂项 / 已知缺口

- **timeline.csv 导出**：`timeline_csv()` 已在 core 实现，但还没接到命令/导出；加一个 `sop run timeline <sop> <run_id>` CLI 子命令（或 app 命令）即可。
- **next-case 显示 bug**：代码修复已就位（`App.svelte` `nextCaseTitle` + `ExecutionView` 按钮用 `nextCaseTitle ?? caseTitle`），svelte-check 通过。若实际运行仍显示旧名，是前端未重新构建/未重载 —— 本会话已重新 build 过 UI，请用新构建验证。
- **文档对齐（4.4）**：DESIGN 仍引用旧 crate 名/命令、D10 与 DESIGN 对 `attachments/` vs `logs/` 说法不一致 —— 未动。
- 新增了 `docs/HANDOFF-tracking.md`；旧的 review 文档仍存在，未合并进 ROADMAP。

## 关键文件

- `crates/sop-core/src/run.rs` — 事件、重放、时间窗合成、`render_record`/`timeline_csv`。
- `crates/sop-core/src/timestamp.rs` — `epoch_seconds`/`rfc3339_from_epoch`/`format_duration`。
- `crates/sop-repo/src/run.rs` — `record`（打时间戳）、`attach`（CSV 时间扫描）、`load`（merge adhoc）。
- `crates/sop-core/src/check.rs` — `missing_step_opened`/`time_regression`/`log_overlap`/adhoc 合法化。
- `crates/sop-app/src/commands.rs` — `run_add_step`、`build_run_view`、`attachment_view`。
- `crates/sop-repo/tests/fixtures/runs/*.md` — 已为现有 result 块补 `opened_at`/`ended_at`（modern 格式）。