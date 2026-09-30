# 需求评估与设计：现场测试记录（上传 / 目录 / 打开 / 设备 / 汇总导出）

> 对象：`SOP-main`（Tauri 2 + Svelte 5 + Rust workspace）
> 依据：通读了 `sop-core` / `sop-repo` / `sop-app` 的相关源码与 `docs/DECISIONS.md`、`docs/LOGS.md`、`docs/HANDOFF-tracking.md`。
> **只读了代码，没有编译或运行测试。** 文中的行号是本次上传版本的位置，动手前请先 grep 确认。
> 配套文件：`TODO-run-records-export.md`（给 AI 逐项执行的任务清单）。

---

## 1. 结论摘要

六条需求都可做，且方向与现有架构一致（事件溯源 + 记录即证据 + Rust 持有规则、前端只渲染）。
但**需求 1（结束后补传日志/图片/备注）与现有“封存”机制正面冲突**，不先解决就会出现：上传直接报错，或上传成功但校验器报 error。其余需求基本是增量。

| # | 需求 | 现状 | 规模 | 风险 |
|---|---|---|---|---|
| 1 | History 每条记录后补传 log / 图片 / 备注 | 只能在运行中挂附件；运行结束后事件被拒绝 | **L** | **高**（C1–C4，见 §2） |
| 2 | History 表格列对齐 | 每个 test case 一张独立 `<table>`，宽度各自自适应 | S | 低 |
| 3 | 设置里增加“其他设备” | 只有 `sensors`（型号+序列号） | M | 低（`hardware` 字段已是自由文本列表） |
| 4 | 实验记录顶层文件夹 + 每个 case 子文件夹 | **已基本存在**：`runs/<sop_id>/<run_id>/`，缺 `photos/` `attachments/` `notes` | S–M | 低（只要不搬迁 `runs/`） |
| 5 | History 一键打开记录/图片/log 所在文件夹 | 无；也没有 opener 插件 | S | 低（注意 AppImage 环境变量） |
| 6 | Summary export = 汇总表 + 各 run 文件夹打包到带时间的导出目录 | 只导出一个 md（coverage 表 + runs 表），列与 History 不一致 | M–L | 中（本地时间、部分失败、大文件） |

建议执行顺序：**A 基础（需求 1 后端 + 4）→ B 命令 → C History UI（1/2/5）→ D 设备（3）→ E 导出（6）→ F 文档与验证**。详见 TODO 文件。

---

## 2. 现状中的关键事实与冲突

### 2.1 目录已经是“每次执行一个文件夹”

```
runs/<sop_id>/<run_id>/     events.jsonl  snapshot.md  record.md  logs/
runs/<sop_id>/<run_id>.md   已提交的正式记录（校验器、manifest、History 都靠它发现 run）
runs/_inbox/                现场观察，不是 run
```

- `<sop_id>` 就是 test case 的 `sop_id`（`crates/sop-repo/src/testplan.rs`），所以“每个 test case 一个子文件夹、其下每次执行一个文件夹”已经成立。
- 路径来源集中在 `crates/sop-repo/src/run.rs`：`run_dir`、`committed_record_file`、`run_paths`（约 L151–160、L981）。
- `runs/` 这个字符串被硬编码在 `lib.rs`（discover）、`validate.rs`（Kind 判定、`expected_place`）、`sop-cli/src/main.rs`（deviations / export 的前缀）、`run.rs`（`all_runs`）。**因此不建议改名或搬迁顶层目录**（见决策 Q1）。

### 2.2 四个冲突（需求 1 的前置问题）

| ID | 事实 | 位置 | 后果 |
|---|---|---|---|
| C1 | run 结束后 `apply` 只放行 `RunConcluded`，其余事件返回 `RunError::Ended` | `sop-core/src/run.rs` 约 L416–418 | 结束后 `attach` / 加备注全部失败 |
| C2 | `end()` 把**整个** `events.jsonl` 的 sha256 写进记录 `events_sha256`；校验器对整个文件重算，不一致即 **error** | `sop-repo/src/run.rs::end`（约 L914–941）、`validate.rs` 约 L435 | 就算放开 C1，补传后立刻校验失败 |
| C3 | `declared_outputs_present`：`status: complete` 的 run 缺任何步骤声明的 `outputs:` 文件就报 **error** | `sop-core/src/check.rs` 约 L907 | 日志事后才到，结束时必然缺 → 误报。当前 `testplan/` 里的步骤没有声明 `outputs:`，所以暂时没触发，但规则在 |
| C4 | 被校验的正式记录 `<run_id>.md` 只在 `end()` 写一次；之后的事件只重写 `record.md`（`record_events` → `write_record`） | `sop-repo/src/run.rs` | 结束后补传的附件不会进入 `logs:` 前言，也不会被校验（哈希、大小） |

另有一个相关的既有小问题：结束后单独追加 `RunConcluded` 同样会破坏封存哈希（C2 的同一原因），本次改成前缀封存后一并解决。

### 2.3 其他相关事实

- 附件目前只有 `logs/` 一个目录，同名不同内容加 `-2` 后缀，同内容去重（`attach` / `attachment_dest`）。
- `run_attach` 命令和前端 `open({multiple:false})` 都是**单文件**（`api.ts` L168 附近）。
- `AttachmentAdded` 事件没有“类型”字段；前言 `logs:` 里每项有 path / sha256 / size / 时间范围 / step。
- 备注：`NoteAdded` 事件，Markdown，渲染进 record（D21）。
- `hardware` 是 `RunStarted` 里的 `Vec<String>` 自由文本；开始表单已有“勾选 checklist 的 equipment + 额外文本”。`sensors` 设置存在 `Settings.extra` 里，字符串格式 `model: serial, serial; model`，**Rust（`sop-core/src/settings.rs`）和 TS（`ui/src/lib/sensors.ts`）各有一份解析器**。
- 时间：事件时间戳统一 UTC，由 Rust 打（`timestamp.rs` 明确“不引入 chrono”）；前端 `dates.ts` 转本地时间显示。所以 Rust 侧生成的汇总表目前没有本地时间。
- 导出：`exports/` 已在 `.gitignore`。仓库根目录里有两个疑似上次导出留下的文件（`test-summary.md`、`heading-error-2026-09-29-heading-error-uas-mag.md`），是“另存为”对话框默认落在根目录造成的；需求 6 落地后建议手动删除，别提交。
- Tauri 只装了 `tauri-plugin-dialog`；`capabilities/default.json` 只有 `core:default` + `dialog:default`；CSP 是 `img-src 'self' data:`（所以不能直接在窗口里显示本地照片，见 §4.5）。
- `ui/dist` 会被 Tauri 直接嵌入；改完前端必须 `npm run build`，否则运行的还是旧界面（`HANDOFF-tracking.md` 里已经踩过一次）。

---

## 3. 目录设计（需求 4）

保持 `runs/` 为唯一顶层记录目录，只在每次执行的文件夹里扩展：

```
runs/
  <sop_id>/                          ← 一个 test case
    <run_id>.md                      已提交正式记录（保持原位，发现/校验依赖它）
    <run_id>/                        ← 该 case 的一次执行，自包含
      events.jsonl                   事实来源（追加，不改写）
      snapshot.md                    开始时冻结的 checklist
      record.md                      生成物：完整记录
      notes.md                       生成物（新增）：全部备注按时间的汇总，由 NoteAdded 渲染
      logs/                          仪器日志（已有）
      photos/                        照片（新增，按需创建）
      attachments/                   其他附件：pdf/xlsx/截图导出等（新增，按需创建）
```

要点：

- `notes.md`、`record.md` 都是**生成物**，事实来源始终是 `events.jsonl`；不要让用户或代码直接编辑它们。
- 子目录**按需创建**，旧 run 不需要迁移；已有 `logs/` 路径与记录里的 `path` 不变。
- 删除 run 已经是 `remove_dir_all(run_dir)`，新子目录自动随之删除，不用改，只补测试。
- 备选方案（**不推荐**）：把顶层改名为 `records/` 或把 `<run_id>.md` 挪进 `<run_id>/`。要同时改 discovery、校验器、CLI、`all_runs`、`delete`、fixtures 和旧数据迁移，风险远大于收益。若你确实要这样，请单独立项。

---

## 4. 各需求设计

### 4.1 需求 1：History 补传 log / 图片 / 备注

**数据模型**

- `AttachmentAdded` 增加可选字段 `kind`：`"log" | "photo" | "file"`，缺省视为 `log`（旧事件不变）。目录映射：`log→logs/`、`photo→photos/`、`file→attachments/`。
- 前言 `logs:` 列表保持同一个键，每项增加 `kind:`（缺省 `log`）。不新开键，避免两处读取逻辑。
- 备注沿用 `NoteAdded`（运行级，`step` 为空）。

**结束后允许的事件**（白名单）：`AttachmentAdded`、`NoteAdded`、`RunConcluded`（已有）。其余事件仍返回 `Ended`。回放时把“在 `RunEnded` 之后应用的事件”标记为 `post_run`，记录里单独渲染一节 **“Added after the run ended”**（带时间），审阅者能区分“现场记录”和“事后补充”。

**封存改为前缀封存**（解决 C2）

- `end()` 记录 `events_sealed_bytes: N`（结束时 `events.jsonl` 的字节数）和 `events_sha256`（**前 N 字节**的哈希）。
- 校验器：有 `events_sealed_bytes` → 只哈希前 N 字节；没有（旧记录）→ 哈希整个文件（行为不变）。第 N 字节之后的行必须都是白名单事件，否则 error。
- 对旧记录第一次补传：若前言有 `events_sha256` 但没有 `events_sealed_bytes`，追加前先把当前文件长度作为 N 写入。
- 这样保留了“现场记录被事后篡改可检测”的原意，同时允许事后追加。

**正式记录同步刷新**（解决 C4）

- 每次事后事件成功后，重新生成 `<run_id>.md`（复用 `run_record_text`，seal 字段沿用已有值），同时刷新 `record.md` 和 `notes.md`。
- 用 `atomic::write`，与现有写入方式一致。

**declared outputs**（解决 C3）：改为“按最终状态（含事后附件）计算”，并把 `complete` 但缺输出从 **error 降为 warning**（`declared_output_pending`），文案提示“等待日志上传”。History 用文件计数列让人一眼看出哪些 run 还没传日志。

**UI（History）**：每行点开一个详情抽屉（见 4.2），里面：`Add log…`、`Add photos…`、`Add files…`（都支持多选）、`Add note…`（文本框）、已有文件列表（类型 / 名称 / 大小 / 添加时间 / “after run” 标记）。批量上传逐个处理，单个失败不影响其他，最后汇总提示。

**不做**：单个附件的删除/替换（证据只增不改）。传错了先靠备注说明；整 run 删除仍可用。若你需要，另加 `AttachmentRemoved` 事件（见决策 Q6）。

**体积**：手机照片单张几 MB，`git push` 会把它们带上。UI 上对 >50 MB 的单文件给确认提示；`docs/LOGS.md` 补一段照片的 LFS 建议（`runs/**/photos/**`）。默认不启用 LFS（与现有政策一致）。

### 4.2 需求 2：History 列对齐

**根因**：`HistoryView.svelte` 里 `runTable` 对每个 case 渲染一张独立 `<table style="width:100%">`，自动布局各自按内容算列宽，所以不同 case 之间列对不齐。

**方案**：`table-layout: fixed` + 共享 `<colgroup>`（列宽只定义一处）+ 文本列 `nowrap / ellipsis` 并用 `title` 显示全文；外层容器 `overflow-x: auto`（窗口最小 1100px）。操作列缩成一个 `Details ▸` 按钮，其余操作（上传、打开文件夹、导出记录、删除）放进详情抽屉的 `<tr class="detail"><td colspan=N>`，这样列不会因为按钮变多而再次撑乱。

建议列：`run id | started | site | sensor | operator | outcome | conclusion | dev | files | ▸`。其中 `files` 显示 `logs·photos·files` 计数，无日志的已结束 run 显示弱提示（服务需求 1）。

### 4.3 需求 3：其他设备设置

- 保留 `sensors`（它决定 run id、History 的 sensor 筛选和记录里的 `SensorIdentity`），**新增 `devices` 设置**表示 GNSS 接收机、基站、无人机、电池、电脑/地面站、电台、三脚架等。
- 存储沿用字符串：`kind/name: serial, serial; kind/name; ...`（`kind/` 可省略）。名称与序列号中禁止 `; : , /`，设置时校验并报错。
- Rust：`sop-core/src/settings.rs` 增 `DeviceStock{kind,name,serials}`、`parse_devices` / `format_devices`（往返测试）；TS：`ui/src/lib/devices.ts` 镜像实现，**两边使用同一组测试向量**（因为现有 sensors 已经是双份解析，别再放大漂移风险）。
- 设置页：新增 `DevicesEditor.svelte`（按 kind 分组；kind 输入框带 `datalist` 常用值，允许自定义）。
- 开始 run 表单：新增“Equipment used”分组勾选（多序列号时给下拉），结果以 `"<kind>: <name> <serial>"` 追加到现有 `hardware` 列表；原来的 checklist equipment 勾选和自由文本保留。**不改事件结构。**

### 4.4 需求 5：打开文件夹

- 新命令 `open_run_folder(sop, run_id)`：两个 id 都用 `is_valid_id` 校验，路径**只由后端从 id 计算**（前端不能传任意路径）；目录存在则打开它，只有旧式记录没有目录时退回打开 `runs/<sop>/`。
- 实现用 `std::process::Command` 调 `xdg-open`（Linux）/`open`（macOS）/`explorer`（Windows），不阻塞、丢弃 stdout/stderr。不引入 `tauri-plugin-opener`，避免多一个插件和权限范围配置。
- 失败时把完整路径显示在 History 提示条里，方便手动复制。
- 风险：AppImage 里启动 `xdg-open` 可能继承不合适的 `LD_LIBRARY_PATH` 等环境变量；spawn 前清理这些变量，并在 Ubuntu（deb 与 AppImage 各一次）手动验证。

### 4.5 需求 6：Summary export 打包

**产物结构**（默认目标目录为工作副本的 `exports/`，用目录选择对话框可改）：

```
<dest>/export_2026-09-29_143205/        ← 上级文件夹：导出的本地时间
  summary.md                            汇总表
  summary.csv                           同内容（UTF-8 BOM，Excel 直接打开）
  runs/<sop_id>/<run_id>.md             正式记录
  runs/<sop_id>/<run_id>/...            该次测试的整个文件夹（events、record、notes、logs、photos、attachments）
  MANIFEST.sha256                       （P2）包内每个文件的 sha256
```

**汇总表内容**：先保留现有 coverage 表（每个 case 一行，含没跑过的），再写 Runs 表，列与 History 对齐并加上备注和文件夹链接：
`Test plan | Test case | Run ID | Started(本地时间) | Site | Operator | Sensor | Outcome | Conclusion | Deviations | Files(logs/photos/files) | Notes | Folder`。
表头之前加一段元信息：项目名、导出时间（本地）、应用版本、工作副本 git 短哈希及是否有未提交改动。`Folder` 列用相对链接（run id 只含合法字符，链接无需转义），整个包搬到别处仍可点开。

**保证与 History 一致的做法**：Runs 表的行直接由 manifest 的 `RunEntry`（History 用的同一份数据，`manifest.rs` 约 L209–233）生成，再补文件计数与备注，而不是再从 `RunState` 拼一遍（现在 `summary.rs` 缺 operator / conclusion / deviations，正是这样漂移的）。

**本地时间**：Rust 侧目前没有时区转换。建议只在 `sop-app` 引入 `chrono`（`default-features=false`，`clock`），`sop-core` 保持无依赖；把“UTC 字符串 → 本地字符串”的格式化函数作为参数传给 `sop-repo` 的汇总/导出函数，`sop-repo` 仍然不依赖 chrono。导出文件夹的时间标签也用它生成（秒级，避免重名；若目录已存在加 `-2`）。

**可靠性**：先写到 `export_<time>.partial/`，全部成功后重命名；失败则删除 partial 并报告。逐文件复制、只复制普通文件、不跟随符号链接；复制后校验大小。返回“复制了多少个 run / 文件 / 字节、跳过或失败了什么”，UI 显示，并给一个 `Open export folder` 按钮（复用 4.4）。

**逻辑放置**：新增 `crates/sop-repo/src/export.rs`（与 `summary.rs` 同层），`sop-app` 命令只做参数校验和调用，符合“规则在 core/repo、app 是薄壳”的既有约定；CLI 同步提供 `sop export --out DIR`，便于测试和 CI。

**照片显示**：CSP 只允许 `img-src 'self' data:`，本次**不在窗口里显示缩略图**，只列文件名和计数，用“打开文件夹”看图。要缩略图需要 asset 协议或后端转 data URL，另立项。

---

## 5. 决策与待确认（TODO 按“默认值”执行）

| ID | 问题 | 默认 | 备选 / 代价 |
|---|---|---|---|
| Q1 | 顶层目录 | 继续用 `runs/`，只扩展子目录 | 改名 `records/`：要改 discovery、校验器、CLI、`all_runs`、`delete`、fixtures，并迁移旧数据 |
| Q2 | 事后追加 vs 封存 | 前缀封存 + 白名单事后事件 | 每次追加后重新封存：实现更简单，但“现场记录未被篡改”的证明变弱 |
| Q3 | 汇总/导出里的本地时间 | `sop-app` 引入 chrono，格式化函数注入 | 前端传时区偏移（夏令时会错）；或汇总里保留 UTC 并注明 |
| Q4 | complete 但缺声明输出 | error → warning | 保持 error：则“先结束、后传日志”的流程会持续报错 |
| Q5 | 导出范围 | 全部 run | 只导出当前筛选结果：coverage 表语义变复杂，可作为后续开关 |
| Q6 | 传错文件如何处理 | 本次不支持删除单个附件，用备注说明 | 新增 `AttachmentRemoved` 事件（只标记，不物理删除） |
| Q7 | 照片进 git | 直接提交 + 文档写 LFS 建议 | 默认启用 LFS：clone 端没装 LFS 会破坏检出（`LOGS.md` 已说明） |
| Q8 | `devices` 是否取代 `sensors` | 并存 | 合并：要改 run id 生成、History 筛选、旧设置迁移 |
| Q9 | 导出文件夹命名与位置 | `<dest>/export_<YYYY-MM-DD_HHMMSS>/`，默认 dest=`exports/` | 若你想要的是 `export/<时间>/`，只改一处命名常量 |

---

## 6. 风险清单

1. **封存校验回归**：前缀哈希改动会影响所有已结束的旧记录。必须有“旧记录（无 `events_sealed_bytes`）仍按整文件校验通过”的测试，并对 `runs/` 里现有三个真实 run 跑一遍 `sop validate`。
2. **并发写**：每个 run 的事件日志有文件锁（`File::lock`，`Cargo.toml` 注释）；事后追加走同一个 `record_events`，不要绕开。
3. **路径安全**：所有进入路径的 id 走 `is_valid_id`；导出标签、上传文件名不能带路径分隔符；不跟随符号链接。
4. **大文件与内存**：现有 `attach` 是 `fs::read` 整个文件进内存再哈希；照片和长日志会放大内存占用。建议顺手改为流式读取 + 分块哈希（P2，不改变行为）。
5. **本地化**：备注、文件名可能含中文；`summary.csv` 需 UTF-8 BOM；Markdown 表格里的 `|` 和换行要转义（现有 `esc`）。
6. **前端旧构建**：交付前必须重新 `npm run build`。

---

## 7. 验收标准（汇总）

1. 结束 run 后，可在 History 给它补传多个日志、多张照片、其他文件和备注；文件落到对应子目录；`sop validate` 通过（无 error）。
2. 旧记录（无 `events_sealed_bytes`）校验行为不变；被人为改动的“封存部分”仍报 error；封存之后出现非白名单事件报 error。
3. 事后补充在 `record.md` / `<run_id>.md` 里出现在独立小节并带时间；`notes.md` 存在且内容与备注一致。
4. History 中不同 case 的表格列左边界像素级对齐；窗口最窄（1100px）时不撑破布局。
5. `Open folder` 在 Ubuntu deb 与 AppImage 下都能打开对应目录，失败时显示路径。
6. 设置页可增删 devices；开始 run 时可勾选；记录的 `hardware` 中出现对应条目。
7. `Export summary` 生成带本地时间的导出文件夹，含 `summary.md`、`summary.csv`、所有 run 的正式记录和完整文件夹；表格列与 History 一致；中途失败不留半成品。
8. `cargo fmt --check`、clippy、`cargo test --workspace`、`sop validate`、`npm run check`、`npm run build` 全部通过（以 `.github/workflows/validate.yml` 为准）。
