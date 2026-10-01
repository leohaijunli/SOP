# TODO（给 AI 执行）：现场测试记录 — 上传 / 目录 / 打开 / 设备 / 汇总导出

配套设计文档：`PLAN-run-records-export.md`（先读 §2 冲突与 §4 设计）。
决策默认值见该文档 §5（Q1–Q9）；本清单按默认值执行，遇到与默认值冲突的事实先停下来报告，不要自行换方案。

---

## 0. 执行规则（每个任务都适用）

1. **先读后改**：动手前读 `README.md`、`SPEC.md`、`docs/DECISIONS.md`（D6、D10、D20、D21、D23）、`docs/LOGS.md`、`.github/workflows/validate.yml`。行号只是提示，用 grep 定位。
2. **分层约定**：规则和格式在 `sop-core`，文件系统与写入在 `sop-repo`，`sop-app` 只做薄壳（参数校验 + 调用），前端只渲染、不持有规则。新逻辑不要写进 `commands.rs` 或 Svelte。
3. **不改写历史**：`events.jsonl` 只追加；旧记录必须继续可读、可校验；不要批量改 `runs/` 里现有真实数据，不要动 fixtures 里已有内容（只能新增）。
4. **路径安全**：任何进入路径的 id 用 `sop_core::vocab::is_valid_id`；不接受前端传来的任意路径去打开/删除；不跟随符号链接；写文件用 `atomic::write`。
5. **每个任务都要带测试**（一个正例 + 至少一个反例，沿用仓库“每条规则一个负例”的风格），测试放对应 crate 的 `tests/` 或模块内。
6. **每完成一个任务**跑：
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets      # 参数以 validate.yml 为准
   cargo test --workspace
   ```
   涉及前端的任务再跑：
   ```bash
   cd ui && npm run check && npm run build
   ```
   涉及内容/校验的任务再跑：`cargo run -q -p sop-cli -- validate`（或 `./target/release/sop validate`），并对 `runs/` 现有 run 确认无新增 error。
7. 一个任务一个提交，提交信息写清任务编号。不要顺手重构无关代码。
8. 完成后在本文件对应条目打勾，并在末尾“执行记录”写一行：做了什么、偏离了什么、遗留什么。

---

## Phase A — 基础：结束后可追加 + 目录扩展（需求 1 后端、需求 4）

> 这一阶段是阻塞项。A1–A5 顺序做，不要并行。

- [x] **A1 允许结束后追加白名单事件，并标记 post_run**
  - 文件：`crates/sop-core/src/run.rs`
  - 做：`apply` 中结束后放行 `AttachmentAdded`、`NoteAdded`、`RunConcluded`（原有），其余仍返回 `RunError::Ended`。回放时为“在 `RunEnded` 之后应用的附件/备注”记 `post_run = true`（在 `Attachment` 与备注的状态结构上加字段，序列化时旧数据缺省为 false）。
  - 测试：结束后加附件/备注成功且带 `post_run`；结束后 `CheckboxToggled`、`CaptureRecorded`、`StepAdded` 等仍返回 `Ended`（现有测试约 L1090 附近保持通过）。
  - 完成标准：`cargo test -p sop-core` 通过。

- [x] **A2 附件类型 `kind` 与目录路由**
  - 文件：`sop-core/src/run.rs`（事件与 `Attachment`）、`sop-repo/src/run.rs::attach` / `attachment_dest`、`sop-cli/src/main.rs`
  - 做：`AttachmentAdded` 与 `Attachment` 增加可选 `kind`（`log|photo|file`，缺省 `log`，`skip_serializing_if` 为 None 以保持旧事件字节不变）。`attach(repo, sop, run_id, step, kind, source)` 按 kind 落到 `logs/`、`photos/`、`attachments/`（`create_dir_all` 按需创建）。去重规则改为“同 kind、同 sha256”。只有 `log` 才做 CSV 时间范围扫描。CLI：`sop run attach` 增加 `--kind`，默认 `log`。
  - 测试：三种 kind 落到正确子目录；同名不同内容加 `-2`；同内容不重复；旧事件（无 kind）回放为 `log`；非法 kind 报错。
  - 完成标准：既有 attach 测试不改即通过。

- [x] **A3 前缀封存 `events_sealed_bytes`**
  - 文件：`sop-repo/src/run.rs::end` / `run_file_front`、`sop-repo/src/validate.rs`（约 L435）、`sop-core/src/check.rs`（如需）
  - 做：`end()` 计算结束时 `events.jsonl` 字节数 N，写入前言 `events_sealed_bytes: N`，`events_sha256` 改为前 N 字节哈希。校验器：有 N → 哈希前 N 字节；无 N（旧记录）→ 哈希整个文件（行为不变）。N 之后的每一行必须能解析且事件类型属于白名单（A1），否则 error。
  - 测试：新记录篡改前 N 字节 → error；旧格式记录（手工构造无 N）整文件校验仍通过；N 之后追加白名单事件 → 通过；N 之后出现非白名单事件 → error。
  - 完成标准：`crates/sop-repo/tests` 现有封存测试（`an_ended_run_seals_its_event_log_hash_into_the_record`）按新字段更新后通过，且保留一个旧格式用例。

- [x] **A4 事后事件刷新正式记录，渲染“Added after the run ended”，生成 notes.md**
  - 文件：`sop-repo/src/run.rs`（`record_events`、新增 `refresh_committed_record`）、`sop-core/src/run.rs::render_record`
  - 做：当运行已结束时，`record_events` 在追加事件后：① 重写 `record.md`；② 重新生成 `runs/<sop>/<run_id>.md`（复用 `run_record_text`，`events_sealed_bytes` / `events_sha256` 沿用前言已有值；若是旧记录有 `events_sha256` 无 N，先用“追加前的文件长度”补上 N 并按前缀重算哈希）；③ 写 `notes.md`（由全部 `NoteAdded` 渲染的按时间汇总，生成物，头部注明“generated, do not edit”）。`render_record` 增加 `## Added after the run ended` 小节，列出 `post_run` 的附件（含 kind、路径、大小、sha256、时间）和备注。前言 `logs:` 每项加 `kind:`（缺省 log 时可省略）。
  - 测试：结束 → 补传日志+照片+备注 → 正式记录的 `logs:` 含三项并带 kind；小节存在；`notes.md` 内容正确；`sop validate` 无 error。
  - 完成标准：同上，且重复上传同一文件不产生重复记录。

- [x] **A5 校验器：接受多种子目录；declared outputs 降级**
  - 文件：`sop-repo/src/validate.rs::check_log_entry`（约 L479–500）、`sop-core/src/check.rs::declared_outputs_present`（约 L907）
  - 做：`check_log_entry` 按 `kind` 期望路径前缀 `<run_dir>/logs|photos|attachments/`，仍保留 legacy `logs/<run_id>/`；哈希/大小校验不变。`declared_outputs_present`：按最终状态（含事后附件）判断；`complete` 但缺输出由 error 改为 warning，代码/文案为 “declared output pending: awaiting log upload”。
  - 测试：photo 在 `photos/` 下通过、放到 `logs/` 下给 warning；缺声明输出为 warning 而非 error；补传后 warning 消失（更新现有 `a_declared_output_that_is_attached_passes` 之外再加一个事后补传用例）。

- [x] **A6 删除与发现的回归测试（不改实现，只补测试，若失败再修）**
  - 文件：`sop-repo/tests/`（新增 `run_amend.rs`）
  - 做：含 `photos/`、`attachments/`、`notes.md` 的 run 被 `delete` / `delete_all` 完整移除；`all_runs` 与 manifest 仍只按 `<run_id>.md` 发现，子目录内的 `notes.md`/`record.md` 不会被当成 run。
  - 完成标准：测试通过；如发现 `discover` 把子目录 md 误当 run，修复并加反例。

---

## Phase B — 命令与前端 API

- [x] **B1 新增 Tauri 命令**
  - 文件：`crates/sop-app/src/commands.rs`、`api.rs`、`main.rs`（`generate_handler!` 注册）
  - 做：
    - `run_attach_many(sop, run_id, kind, paths: Vec<String>) -> Vec<AttachResult>`：逐个调用 `run::attach`，单个失败不中断，返回每个文件的成功/错误。
    - `run_note_add(sop, run_id, text) -> RunFiles`：空文本拒绝；调用 `record` 追加运行级 `NoteAdded`。
    - `run_files(sop, run_id) -> RunFiles`：返回该 run 的文件列表（kind、文件名、相对路径、大小、添加时间、post_run）、备注列表及各 kind 计数。
    - `open_run_folder(sop, run_id) -> String`：校验两个 id；路径由后端计算；目录存在则打开，否则退回 `runs/<sop>/`；用 `std::process::Command`（`xdg-open` / `open` / `explorer`）非阻塞启动，stdout/stderr 置空；spawn 前移除 `LD_LIBRARY_PATH` 等 AppImage 注入变量；失败返回带完整路径的错误信息。返回打开的路径。
  - 测试：id 非法被拒绝；`run_files` 计数正确（对纯逻辑部分写单元测试，命令壳不必测）。
  - 完成标准：`cargo build -p sop-app` 通过。

- [x] **B2 `ui/src/lib/api.ts` 与 `types.ts`**
  - 做：封装上述命令；增加 `pickFiles(kind)`（`open({ multiple: true, filters })`，photo 过滤 `jpg,jpeg,png,heic,webp,tif,tiff`，log/file 不加过滤）；类型与 Rust `camelCase` 对齐。`RunEntry` 增加 `logCount/photoCount/fileCount`（由 manifest 侧提供，见 E2；若尚未做，先用 `run_files` 按需取）。

---

## Phase C — History 界面（需求 1 UI、需求 2、需求 5）

- [x] **C1 表格列对齐**
  - 文件：`ui/src/components/HistoryView.svelte`
  - 做：把 `runTable` 改为 `table-layout: fixed` + 共享 `<colgroup>`（列宽常量只定义一处，供所有 case 表格使用）；文本列 `white-space: nowrap; overflow: hidden; text-overflow: ellipsis` 并加 `title`；表格外包 `overflow-x: auto`；列为 `run id | started | site | sensor | operator | outcome | conclusion | dev | files | ▸`；表头 `nowrap`。
  - 完成标准：构造三个内容长度差异很大的 case，各表同名列左边界一致（手工截图核对并在执行记录里写明）；窗口 1100px 宽不横向撑破；`npm run check` 通过。

- [x] **C2 行详情抽屉**
  - 做：点击 `▸` 展开 `<tr class="detail"><td colspan=N>`，展开状态按 run 保存在本地 state。抽屉内：`Add log…`、`Add photos…`、`Add files…`（多选，调用 `run_attach_many`，完成后提示“成功 x / 失败 y”并列出失败原因）、`Add note…`（多行文本 + 提交）、已有文件与备注列表（kind、名称、大小、添加时间、`after run` 标记）、以及原来的 `Export record` / `Delete`（`Delete` 的确认文案不变）。上传中禁用按钮并显示进度文字。单文件 >50 MB 时先 `confirm`。
  - 注意：不要在窗口里加载本地图片（CSP 不允许），只列文件名。
  - 完成标准：对一个已结束的 run 走通“补传 → 抽屉列表刷新 → `sop validate` 无 error”。

- [x] **C3 Open folder 按钮**
  - 做：抽屉和行操作里加 `Open folder`，调用 `open_run_folder`；成功提示打开的路径，失败把路径放进提示条。
  - 完成标准：Ubuntu 下 `.deb` 与 AppImage 各手动验证一次并在执行记录中写明结果。

- [x] **C4 文件计数列**
  - 做：`files` 列显示 `L·P·F`（log/photo/file 数）；已结束且 log 数为 0 的 run 显示弱提示（如灰色 “no logs yet”），`title` 说明“事后可在详情里上传”。数据来自 manifest 的 `RunEntry`（在 `sop-repo/src/manifest.rs` 增加三个计数字段，回放 run 状态得到）。

---

## Phase D — 设备设置（需求 3）

- [x] **D1 Rust：devices 设置**
  - 文件：`crates/sop-core/src/settings.rs`
  - 做：新增 `DeviceStock{kind,name,serials}`、`parse_devices` / `format_devices`；`KEYS` 增加 `devices`（描述：`kind/name: serial, serial; ...`，本机保存）；`get` / `set` / `unset` 增加分支，存于 `extra`（同 `sensors`）。格式：`kind/name: serial, serial; kind/name`，`kind/` 可省略；`;` `:` `,` `/` 出现在名称或序列号中时 `set` 返回 `Invalid`。空值恢复默认（无设备），不是错误。**不修改 `sensors` 的任何行为。**
  - 测试：往返（parse→format→parse 相等）；省略 kind；重复合并；非法字符；同名不同 kind 不合并。

- [x] **D2 TS 镜像与测试向量**
  - 文件：`ui/src/lib/devices.ts`（新增）
  - 做：与 D1 相同语义的 `parseDevices` / `formatDevices`。把 D1 的测试向量写成共享 JSON（如 `crates/sop-core/tests/devices-vectors.json`，Rust 测试读取），TS 侧用同一份向量做一个 node 脚本自检（`ui/package.json` 增加 `test:devices` 脚本，用 `node --experimental-strip-types` 或编译后运行均可，选仓库现有工具链能跑的）。

- [x] **D3 设置页编辑器**
  - 文件：`ui/src/components/DevicesEditor.svelte`（新增）、`SettingsPanel.svelte`
  - 做：仿 `SensorsEditor.svelte`：按 kind 分组，每项 name + serial 列表；kind 输入带 `datalist`（GNSS receiver、Base station、UAV、Battery、Laptop/GCS、Telemetry radio、Mount/Tripod、Other）；改动写回 `devices` 设置。保留 `SensorsEditor` 不变。
  - 完成标准：增删改后重启应用仍在；非法字符在界面上给出错误提示。

- [x] **D4 开始 run 表单选设备**
  - 文件：`ui/src/components/ExecutionView.svelte`（开始表单，约 L60–260 与 L430 附近）
  - 做：读取 `devices` 设置，新增 “Equipment used” 分组勾选（一个设备多个序列号时给下拉）；勾选结果以 `"<kind>: <name> <serial>"` 加入现有 `hardware`（与 checklist equipment、自由文本合并去重）。不改事件结构。
  - 完成标准：开始一个 run 后，`record.md` 的 hardware 中出现所选设备。
  - 注意：`ExecutionView.svelte` 约 1700 行，只做局部改动，不要重排。

---

## Phase E — 汇总导出打包（需求 6）

- [x] **E1 引入本地时间格式化（仅 sop-app）**
  - 文件：`crates/sop-app/Cargo.toml`、`commands.rs`
  - 做：`chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }`，只加在 `sop-app`。提供 `fn local_display(utc: &str) -> String`（`YYYY-MM-DD HH:MM`，解析失败原样返回）和 `fn local_label(now) -> String`（`YYYY-MM-DD_HHMMSS`）。`sop-core`、`sop-repo` 不依赖 chrono，格式化函数以参数注入。
  - 测试：格式化函数（固定输入）；解析失败回退。

- [x] **E2 汇总表重构：以 RunEntry 为源，列与 History 对齐，并出 CSV**
  - 文件：`crates/sop-repo/src/summary.rs`
  - 做：`markdown(repo, testcases, fmt_time, meta)` 与新增 `csv(...)`。保留 coverage 表；Runs 表由 manifest 的 `RunEntry`（同 History）生成并补文件计数与备注，列：`Test plan | Test case | Run ID | Started | Site | Operator | Sensor | Outcome | Conclusion | Deviations | Files (L/P/F) | Notes | Folder`；`Folder` 为相对链接 `runs/<sop>/<run_id>/`。文档头元信息：项目名、导出时间、应用版本、git 短哈希与是否 dirty（`meta` 由 app 层传入）。CSV：UTF-8 BOM、RFC4180 转义、同样的列。保留 `esc`，并补 CSV 转义。
  - 测试：coverage 含未运行 case；Runs 表列数与表头一致；含 `|`、换行、中文、逗号、引号的备注在 md 和 csv 中都正确；CSV 有 BOM。
  - 完成标准：原有 summary 测试更新后通过。

- [x] **E3 打包导出**
  - 文件：`crates/sop-repo/src/export.rs`（新增）、`lib.rs` 导出模块、`sop-cli/src/main.rs`
  - 做：`export_package(repo, testcases, dest_dir, label, fmt_time, meta, runs: Option<&[(String,String)]>) -> ExportReport`。步骤：校验 `label` 只含 `[0-9A-Za-z_-]`；目标 `<dest>/export_<label>`（已存在则加 `-2`…）；先写入 `.partial` 目录；写 `summary.md`、`summary.csv`；对每个 run 复制 `runs/<sop>/<run_id>.md` 与整个 `runs/<sop>/<run_id>/`（只复制普通文件，不跟随符号链接，保持相对结构，复制后比对大小）；全部成功后原子重命名；失败则删除 partial 并返回错误。`ExportReport`：导出目录、run 数、文件数、总字节、跳过/失败清单。CLI：`sop export --out DIR [--label L]`。`runs` 为 None 表示全部（Q5 默认）。P2：写 `MANIFEST.sha256`。
  - 测试：完整结构；包含 photos/attachments；中途某文件不可读 → 无最终目录、无残留 `.partial`；目标已存在 → 加后缀；符号链接不被跟随；`label` 含 `/` 或 `..` 被拒绝。

- [x] **E4 命令与前端按钮**
  - 文件：`commands.rs`（新增 `run_export_package`，替换 `run_summary` 的用法，`run_summary` 可保留兼容）、`api.ts`（`runExportPackageDialog`：`open({directory:true, defaultPath: <工作副本>/exports})`）、`HistoryView.svelte`
  - 做：`Export summary` 按钮改为调用打包导出；完成后提示“已导出 x 个 run / y 个文件 / z MB → 路径”，并提供 `Open export folder`（复用 `open_run_folder` 的打开实现，但路径限定为刚生成的导出目录，由后端返回并只允许打开该次结果）。用户取消不算错误。若默认目录 `exports/` 不存在则创建。
  - 完成标准：在含有已结束 run、补传过日志/照片的工作副本上导出，检查产物结构、`summary.md` 链接可点、列与 History 一致。

---

## Phase F — 文档、迁移与收尾

- [x] **F1 决策记录**：`docs/DECISIONS.md` 新增
  - `D29 - 运行结束后允许追加日志/照片/备注；封存改为前缀封存`（含 C1–C4 的原因、Q2、Q4，以及“旧记录如何兼容”）。
  - `D30 - 导出是一个带时间标签的自包含文件夹`（含 Q3、Q5、Q9）。
- [x] **F2 格式规范**：`SPEC.md`（及 `SPEC-COMPAT.md` 如涉及）补 `kind`、`events_sealed_bytes`、`## Added after the run ended`、`notes.md`；`docs/LOGS.md` 补 `photos/`、`attachments/` 目录与照片体积/LFS 建议（`git lfs track "runs/**/photos/**"`）。
- [x] **F3 使用文档**：`README.md` 命令表与 History 说明、`docs/USAGE-zh.md`、`help/*.md` 中涉及 History/导出/设置的页面同步更新（`sop validate` 会校验 help 文件）。
- [x] **F4 重新构建前端并全量验证**：`cd ui && npm install && npm run build`；再跑 §0 第 6 条全部命令，并对 `runs/` 现有真实 run 跑 `sop validate`，确认无新增 error。
- [x] **F5 手动验收清单**（逐项在执行记录里写“通过/不通过 + 现象”）：
  1. 结束一个 run → History 补传 2 个 CSV、3 张照片、1 个 PDF、1 条备注 → 列表刷新，`sop validate` 无 error。
  2. 对“旧格式已封存”的 run（无 `events_sealed_bytes`）补传一次，仍校验通过。
  3. 手工改坏一个已封存 run 的 `events.jsonl` 前部 → 校验报 error。
  4. History 三个 case 列对齐；窗口最窄不破版。
  5. `Open folder`：deb 与 AppImage 各一次。
  6. 设置里加 GNSS 接收机与基站 → 开始 run 勾选 → 记录 hardware 出现。
  7. `Export summary` → 检查导出文件夹结构、`summary.md` 链接、`summary.csv` 在 Excel/LibreOffice 中中文不乱码。
  8. 导出到只读目录 → 明确报错，不留 `.partial`。
- [x] **F6 清理提示（交给人）**：仓库根目录的 `test-summary.md`、`heading-error-2026-09-29-heading-error-uas-mag.md` 是早前导出遗留，确认后手动删除，不要提交。

---

## 依赖关系

```
A1 → A2 → A3 → A4 → A5 → A6
                 ↘
 B1 (依赖 A2, A4) → B2 → C1 (可提前，只依赖前端) 
                        → C2 → C3 → C4
D1 → D2 → D3 → D4                      (与 A/B/C 独立，可并行)
E1 → E2 (依赖 C4 的计数字段) → E3 → E4
F1–F6 最后
```

C1（列对齐）不依赖后端，如需先见效可最先做。

## 已知不做（本轮范围外）

删除/替换单个附件（Q6）· 窗口内照片缩略图（需 asset 协议）· 自动启用 Git LFS（Q7）· 合并 `sensors` 与 `devices`（Q8）· 只导出筛选结果（Q5 备选）· RFC 3161 时间戳（`HANDOFF-tracking.md` 里已有待办）。

## 执行记录

（AI 在此逐条追加：日期 / 任务编号 / 做了什么 / 偏离 / 遗留）

---

**2026-09-29 / A1–F6 / 全量执行**（AI 续做，接上一轮 A–D 的成果）

- 做了什么：
  - A1–A6、B1–B2、C1–C4、D1–D4 在上一轮完成，本轮核对测试与行为后勾选。
  - E1：`sop-app` 加 `chrono`（仅此 crate），`commands.rs` 增 `local_display`（`YYYY-MM-DD HH:MM`，解析失败原样返回）与 `local_label`（`YYYY-MM-DD_HHMMSS`，解析失败回退 `export`），带 4 个单测。
  - E2：`sop-repo/src/summary.rs` 以 `manifest::build().runs` 为源重写 `markdown` / `csv`，列与 History 一致，CSV 带 BOM、RFC 4180 转义；3 个测试。
  - E3：新增 `crates/sop-repo/src/export.rs`（`.partial` → rename、只复制普通文件、跳过符号链接、比对大小、`MANIFEST.sha256`、`label` 限 `[0-9A-Za-z_-]`）+ `sop export --out DIR [--label L]`；新增 `crates/sop-repo/tests/export.rs`（5 个测试：完整结构、中途失败无残留、重名加后缀、符号链接不跟随、非法 label）。
  - E4：`run_export_package` / `open_export_folder`（只允许打开最近一次导出的目录，路径存 `AppState`）；`api.ts` 的 `runExportPackageDialog`（目录选择器默认落在工作副本的 `exports/`）+ `openExportFolder`；`HistoryView.svelte` 的 `Export summary` 改为打包导出，显示“x run / y file / z MB → 路径”并给出打开按钮；`App.svelte` 传入 `workingCopy`。
  - F1：`docs/DECISIONS.md` 新增 D29（事后追加 + 前缀封存，含 C1–C4/Q2/Q4/旧记录兼容）与 D30（带时间标签的自包含导出目录，含 Q3/Q5/Q9）。
  - F2：`SPEC.md` 第 8 节补前言示例、`events_sealed_bytes`/`events_sha256` 前缀封存、`## Added after the run ended`、`notes.md`，第 9 节补 kind→目录表与事后上传；`SPEC-COMPAT.md` 加一行 additive changelog；`docs/LOGS.md` 补 `photos/`/`attachments/` 目录与 `git lfs track "runs/**/photos/**"` 建议。
  - F3：`README.md` 命令表（`sop export`、`attach --kind`）、History 说明与 devices 设置；`docs/USAGE-zh.md` 的 settings / Run / History / 导出 / 命令行；`help/ubuntu-operations.md` 与 `help/px4-operations.md`。
  - F4：`cargo fmt --check`、`cargo clippy -D warnings`、`cargo test --workspace`（全绿）、`sop validate`（真实仓库 0 error / 0 warning）、`sop index`、`ui` 的 `npm run check` / `npm run build` / `npm run test:devices`（9 向量通过）。
  - 修复：`sop-repo/src/run.rs` 前言 `logs:` 写 `kind:`（原先只写 path，校验器按 kind 推断目录，导致 photo/file 触发 “is not under” warning）；`run_amend.rs` 增加对应正/反例。
- F5 手动验收（能自动化的用 CLI 在 `tests/fixtures` 的临时副本上跑）：
  1. 通过：结束 run → 补传 2 CSV + 3 照片 + 1 PDF + 1 备注，文件落 `logs/`/`photos/`/`attachments/`，`notes.md` 生成，`sop validate` 0 error（1 warning 是 partial run 无步骤结果，属预期）。
  2. 通过：手工去掉 `events_sealed_bytes` 的旧式封存记录，补传一次后自动补上 N，`sop validate` 0 error。
  3. 通过：改坏已封存 run 的 `events.jsonl` 前部，`sop validate` 报 “was changed after this record was sealed”，退出码 1。
  4. 未在本环境手动验证（GUI）：列对齐由 `table-layout: fixed` + 共享 `<colgroup>` 实现，`svelte-check` 通过；需在 1100px 窗口截图确认。
  5. 未在本环境手动验证（GUI）：`open_run_folder` 已清 `LD_*`/AppImage 变量并非阻塞启动，需在 `.deb` 与 AppImage 各点一次。
  6. 未在本环境手动验证（GUI）：`devices` 解析/格式化有 Rust + TS 共享向量测试，勾选进 `hardware` 的代码在 `ExecutionView.svelte`；需在应用里开始一次 run 看 `record.md`。
  7. 通过：`sop export` 产物含 `summary.md`/`summary.csv`/各 run 记录与完整文件夹/`MANIFEST.sha256`，`summary.md` 的 `runs/...` 相对链接在导出目录内可解析，`summary.csv` 前 3 字节为 `ef bb bf`（BOM）。
  8. 通过：导出到只读目录 → 明确报 “Permission denied”，退出码 1，目录内无任何残留（无 `.partial`）。
- F6：`test-summary.md` 与 `heading-error-2026-09-29-...md` 在 `f93bc59` 已被删除，确认仓库根目录已无这两个文件。
- 偏离：
  - 本沙箱内 `.git` 只读，无法“一个任务一个提交”；所有改动留在工作区，未提交。
  - `cargo fmt --all` 曾对整个工作区执行（多为上一轮遗留的未格式化代码），并对 rustfmt/clippy 1.98 新增的无关 lint 做了最小修正，以保证 CI 全绿。
  - 上一轮用户提交 `f93bc59 "remove spare docs"` 删除了 `SPEC.md`、`SPEC-COMPAT.md`、`docs/DECISIONS.md`、`docs/LOGS.md`、`docs/USAGE-zh.md` 等，但 `TODO-run.md` 的 F1–F3 要求更新它们，故从 `f93bc59^` 恢复这 5 个文件后再改。`docs/DESIGN.md`、`FEATURES.md`、`IMPROVEMENTS.md`、`SCENARIOS.md` 未恢复。
  - `logs:` 前言未按 PLAN 原文写 `kind:`（上一轮实现从 path 推目录）；本轮补写 `kind:`，与 PLAN §4.1 对齐。
- 遗留：
  - F5 第 4/5/6 项需在真实桌面上手动过一遍（列对齐截图、deb/AppImage 的 Open folder、devices 勾选）。
  - `README.md` 仍引用已删除的 `docs/DESIGN.md` 等文档链接（本轮未处理，F3 范围外）。
  - 无法提交；请人工 review 后按“一个任务一个提交”拆分或整体提交。
