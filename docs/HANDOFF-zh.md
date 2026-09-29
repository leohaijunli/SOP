# 交接说明（中文）

本文件记录**刚提交的这一批**改动、验证到什么程度、以及下一步怎么走。格式的权威
说明是 `SPEC.md`，设计取舍编号索引是 `docs/DECISIONS.md`（Batch 1–2 补到 D27），功能优先级表
是 `docs/FEATURES.md`，本批的评审与路线图是 `docs/REVIEW-engineer-workflow.md`（Phase 1–5，
Phase 6/7 为建议）。中文操作教程在 `docs/USAGE-zh.md`。

> 上一批三个提交（`4766953` Phase 1–3 + note 编辑器，`d0f37cc` Markdown 渲染器 Rust 化，
> `b74cea4` Batch 2：现场可用性）都已推送到 `origin/main`。**当前这一批（Batch 3：
> 追踪价值）见 §0**。工作副本里未提交的只剩 `runs/` 下 App 正在生成/删除的 run，**不要顺手提交**。
> 上一版交接（Batch 2）在历史里：`git show b74cea4:docs/HANDOFF-zh.md`。

## 0. 最近三批（Batch 1：数据安全 + 录入正确；Batch 2：现场可用性；Batch 3：追踪价值）

按 `docs/ROADMAP.md` 的 P0→P1→P2→P3 顺序做的。**Batch 1 做完了 0.2 / 0.3 / 0.4 / 1.1–1.4；
Batch 2 做完了 2.1–2.6；Batch 3 做完了 3.1–3.3**（并捎带 4.1 里把结论词表放进 core）。0.1 需要真机，
留在这里（§0.5）。Batch 3 的改动在 §0.13–§0.15。

### Batch 1

### 0.1 事件日志改成真正的 append（ROADMAP 0.2）

- `crates/sop-repo/src/run.rs`：新增 `append_event`。`run::record` 不再读全文件再用
  `atomic::write` 整体重写，而是 `OpenOptions::append(true)` 追加一行，
  写前 `File::lock()` 上 per-run 锁，写完 `sync_data()` 落盘再解锁。
- 因为 `File::lock` 是 1.89 才进 std 的，workspace 的 `rust-version` 从 `1.88` 抬到
  `1.89`（见到 D23）。
- 结果：日志在**任意字节**被截断，replay 都只丢最后一条被截断的事件，前面的照常回放；
  App 和 CLI（或两个窗口）并发写同一个 run 不会互相覆盖、不会写出半个 JSON 行。
- 新增测试（`crates/sop-repo/tests/run.rs`）：
  `a_log_truncated_anywhere_replays_as_a_prefix`、`concurrent_writers_do_not_lose_events`
  （8 个线程并写，断言每条完整事件都在、且每行都能解析）。

### 0.2 原子写补 fsync（ROADMAP 0.3）

- `crates/sop-repo/src/atomic.rs`：临时文件改成 `OpenOptions` 打开 → `write_all` →
  `sync_all` → `rename` → 再 fsync 父目录。断电后 rename 落盘、内容完整。
- `.gitignore` 加 `*.tmp`（崩溃可能留下原子写的临时文件）。
- 新增测试 `crates/sop-repo/tests/atomic.rs`：写完不留临时文件、父目录会被创建、覆盖写
  用的是全新内容。

### 0.3 结束 run 要先确认（ROADMAP 0.4）

- `ui/src/components/ExecutionView.svelte`：`end()` 拆成 `requestEnd` / `confirmEnd` /
  `cancelEnd`，中间多一条确认条（`endPending`，`Esc` 取消）。三个 `End:` 按钮都改走它。
- `ctrl+enter` 只在**光标不在输入框**时才请求结束；在备注编辑器里它仍然是"保存备注"
  （`MarkdownEditor` 自己处理并 `preventDefault`，窗口层再靠 `inField` 兜底），不会误结束。
- `End: complete` 置灰时的 `title` 直接说明是哪几步挡着（复用 `blockedReason`）。

### 0.4 录入正确性（ROADMAP 1.1–1.4）

- 新增纯函数模块 `ui/src/lib/capture.ts`：`expectsNumber` / `expectedHint` / `expectedOk`
  / `formatProblem` / `missingRequired`。范围判断从组件里搬出来，`expectedOk` 修掉了
  `NaN → true`（`12a` 不再被当成合格）。
- 新增 `ui/src/lib/dates.ts`：`localDay` / `localDateTime`。History 和 Test Plans 显示
  操作员本地时间（UTC 原值放在 `title` 里），修掉"傍晚的 run 显示成第二天、和 run id 对不上"。
- `ExecutionView.svelte`：capture 输入框上方常显 `expected:` 提示；`number`/`integer`
  用 `type="number" inputmode="decimal"`（整数 `step="1"`）；格式不对时显示警告；
  `Mark done` / `space` / capture 最后一个 `Enter` 都走新的 `markDone`——若有空的
  `required` capture，先提示"还有 N 项为空"并要求写一句原因，写进 `StepStatusChanged.reason`
  （`done` 带 reason 是格式允许的，不需要改 SPEC），操作员仍可取消。

### 0.5 真机待办（ROADMAP 0.1，`[verify]`）

以下三件事只是静态阅读推断，**没有在真机上验证**，接手后请在 Ubuntu + `.deb` 上跑一遍
并把结论补进本文件：

1. `prompt()` / `confirm()` 在 Tauri 打包后的 webview 里是否正常弹出（Batch 2 的 2.1
   已经用应用内模态替换了它们；这里只是确认旧行为，作为记录）。
2. 断网启动是否会冻结：启动时无条件 `git pull` 且没有超时（对应 2.2）。
3. 录入中途 `kill -9`、以及拔电之后，run 能否恢复（现在有 append + fsync + 截断容错，
   预期可以，但需要实测确认）。

### 0.6 应用内原因对话框（ROADMAP 2.1）

- 新增 `ui/src/components/PromptModal.svelte`：原生 `<dialog>`（自带焦点陷阱和 `Esc`），
  常用理由是一个按钮点一下，另配一个文本框；`Enter` 提交、`Shift+Enter` 换行。
- `ExecutionView.svelte` 里 4 处 `prompt()` 全部换掉：Skip、Deviate、超范围 Acknowledge、
  空必填项提醒。调用方式仍是 `const reason = await ask({...})`，UI 状态在 `promptRequest`。
- 对话框打开时 `onKey` 直接返回，不影响对话框自己的键盘处理。

### 0.7 命令不再阻塞窗口 + git 超时（ROADMAP 2.2）

- `crates/sop-app/src/commands.rs`：45 个命令全部加上 `#[tauri::command(async)]`。
  Tauri 把同步命令放在窗口主线程上；`(async)` 会把它放到阻塞线程池（等价于
  `spawn_blocking`），函数本身仍是同步的，Rust 侧可以直接调用（见 D25）。
- `crates/sop-repo/src/git.rs`：`git` 子进程有了截止时间（本地查询 15s、联网 30s）。
  新辅助函数 `output_with_timeout` 在旁路线程里读 stdout/stderr（避免输出塞满管道死锁），
  到点 `kill` 子进程并报 `GitError::TimedOut`。3 条单元测试覆盖：正常、超时被杀、输出
  超过管道缓冲不阻塞。
- `ui/src/App.svelte`：启动同步前先看 `navigator.onLine`，断网直接跳过并显示
  "offline - skipped sync"；联网时 git 超时是兜底。

### 0.8 进度与导航（ROADMAP 2.3）

- 运行页顶部 `N / M done` 进度条（`done`、`deviated` 都算有结果）。
- `markDone` 成功后调用 `advance()`：跳到下一个还没结果的步骤，跑完绕回第一个。
- 切换步骤时（`$effect` 里用 `tick()` 等 DOM 更新后）聚焦该步第一个还没填的 capture。
- `End: complete` 置灰时，左侧写明还有几步挡着，并给一个 **Go to &lt;步骤名&gt;** 按钮。

### 0.9 没做完的 run 全局可见（ROADMAP 2.4）

- `ExecutionView.svelte` 的开始表单顶部新增 "Runs still open (N)"：从 `manifest.runs`
  里筛出没有 `status` 的 run，跨 checklist 列出，带 Resume 按钮。
- 属于当前 checklist 的直接 `runState` 打开；属于别的 checklist 的通过新 prop
  `onResumeRun(sopId, runId)` 交给 `App.svelte`，由它切 checklist、置 `resumeRunId`，
  `ExecutionView` 的 `$effect` 再加载，加载完回调 `onResumed()` 清掉。
- 外部 checklist（"Open file"打开、不在 manifest 里的）如果没先加载，会提示先打开它。

### 0.10 顺手修掉的一个真 bug

写并发测试时暴露出 `atomic::write` 的临时文件名是固定的 `path.tmp`：两个线程同时写
`record.md` 时共用一个临时文件，后一个 `rename` 报 "No such file"（`concurrent_writers`
测试先过了一次，再跑就红——正是竞态）。临时名现在带进程号和自增计数（见 D23 末尾）。

### 0.11 开始表单折叠（ROADMAP 2.5）

- 表单上只留必填的 run id / operator / site；仪器、设备、条件、附加硬件收进
  `<details class="more">`，摘要读作 `Same as last run: <machine>: …
  · N equipment · M conditions`（`inheritedSummary` 这个 `$derived` 现算）。
- 切换 checklist 时重跑 seeding（`untrack` 包住，避免自触发）：从该 checklist 自己上一次
  run 回填，并决定这行是折叠还是展开；没有历史可沿用就自动展开。
- 草稿 checklist 的 override reason 保持在折叠之外，因为它每次都必须填。
- `newRun()`（"Start new run"）会重跑同一段 seeding，摘要行跟着刷新。

### 0.12 导航重排（ROADMAP 2.6）

- `ui/src/App.svelte` 的初始 `view` 从 `"run"` 改成 `"testplans"`——默认落在 Test Plans。
- `Header.svelte` 顶栏顺序改成 **Test Plans / Run / History / Browse**；**Edit SOP /
  Project / Settings** 收进右侧的 **Manage ▾** `<details>` 下拉，点击后自动收起。
  三个"管理"视图当前激活时下拉标题高亮。
- 相关中文文档（`docs/USAGE-zh.md`）里 "点工具栏 Project / Settings" 已改成
  "Manage ▾ → …"。

### 0.13 一次 run 的结论（ROADMAP 3.1）

- **记录格式（增量式，schema 仍为 1）**：`runs/<sop>/<run>.md` 的 front matter 新增可选的
  `conclusion`（`pass` / `fail` / `inconclusive`）和 `conclusion_note`（一句话）；事件日志新增
  `RunConcluded { at, conclusion, summary }`。旧日志（没有这条事件）照常回放，旧 reader
  对未知 front matter 键警告并保留（`SPEC-COMPAT.md` 已登记）。
- **语义**：`status`（`complete`/`partial`/`aborted`）只表示"跑完了"；`conclusion` 是操作员
  自己给的判定，工具从不算出它（D4，新 D28）。`conclusion` 与 `conclusion_note` 成对出现，
  且都要在 `status`/`ended` 之后——校验器按"成对 + 已结束"检查（`sop_core::check` 的
  `run_front_matter`）。
- **词表**：`vocab.rs` 新增 `RUN_CONCLUSION`，命令/CLI/API/manifest 全链路上带。
- **CLI**：`sop run end <sop> <run> <status> [--conclusion X --note "…"]`；`--conclusion`
  与 `--note` 必须成对。
- **窗口**：结束 run 的确认条确认后，先问 verdict（`pass`/`fail`/`inconclusive` 一键选），
  再写一句原因；两个都取消/留空则照旧结束（结论是"offer"，不强求）。`api.runEnd` 带
  `conclusion` / `conclusionNote`。
- 测试：`sop_core` 的 3 条（结论须在结束之后、词表 + 必须有一句、记录正文回显）；`sop-repo`
  1 条集成（front matter + 正文 + 校验通过 + manifest 带结论）。

### 0.14 覆盖矩阵（ROADMAP 3.2）

- `TestPlansView.svelte` 的表格从"每个 case 一行状态"改成 **case × sensor serial** 矩阵：
  每列是一个序列号（没记 sensor 的 run 归入 `none` 列），每个格子里是该 case + 该序列号的
  **最近一次 run 的结论**（`pass`/`fail`/`inconclusive`/`in progress`/`—`）。列顺序按 run
  出现顺序稳定生成，避免新增一条 run 时格子跳位。
- manifest 的 `RunEntry` 新增 `conclusion`，让看板不用打开记录就能给格子着色。

### 0.15 历史过滤（ROADMAP 3.3 的前半）

- `HistoryView.svelte` 的过滤从 site/operator/outcome 扩到 **sensor serial / conclusion /
  仅看有偏差（has deviations）**；表格新增 sensor 列和 conclusion 徽章。**并排比较同一 case
  的多次 run、随时间看趋势**这一半留到后面 batch（ROADMAP 3.3 已注明）。

### 0.16 文档同步

- `SPEC.md`（§2 词表 + §8 run 记录）、`SPEC-COMPAT.md` 变更表、`docs/ROADMAP.md`（3.1–3.3 标
  done，进度段更新）、`docs/DECISIONS.md`（D28）、`docs/FEATURES.md`（C/G 段新增三行）、
  `docs/USAGE-zh.md`（结束 run 先问结论）都已更新。

## 1. 当前状态

```
cargo test                  18 个测试目标 / 164 个测试全部通过（Batch 1 +2，Batch 2 +3）
cargo check -p sop-app      通过
sop validate                0 error(s), 0 warning(s)；若 App 里有一条已结束但没记任何结果的
                            run，会多一条 "no step results" warning，这是预期行为
cd ui && npm run check      0 errors, 0 warnings
cd ui && npm run build      通过（ui/dist 已重建，且被 git 跟踪）
git                         上一批已推送；Batch 1 见 §0，提交后一并推送
```

**Batch 1–2 的验证边界**：Rust 侧（append / fsync / 并发 / git 超时）有测试；前端
`capture.ts` / `dates.ts` 是纯函数但没有前端测试框架（`npm install` 装不了 vitest，见 §4），
只过了 `svelte-check`。**应用内改变（模态、自动前进、恢复列表、异步命令）没有在真实
窗口里点过**（导航重排、开始表单折叠同样如此），`cargo check`/`npm run check`/`build`
只能保证类型和编译。真机三件事见 §0.5，另外建议按 §5 的清单把运行页和导航完整走一遍：
启动落在 Test Plans、Manage 下拉、以及开始表单的 "Same as last run: …" 这一行。

**验证到哪一步**：Rust 侧全部有测试；前端只有 `svelte-check` + `build` 的静态验证，
**没有在运行中的窗口里点过**。接手第一件事建议 `REBUILD_UI=1 ./run-app.sh`，按 §5 的
清单走一遍。

## 2. 本批改动（按主题）

### 2.1 Phase 1 — 计划/用例来源变成一等公民

- `RunEvent::RunStarted` 新增可选 `plan` / `case`（`skip_serializing_if`，旧日志没有这两
  个键也能回放）；`RunState`、记录 front matter、`RunMetaInput` / `RunEntry` / `RunView`
  同步跟上。`SPEC-COMPAT.md` 有对应条目。
- "Run plan"：`ui/src/App.svelte` 的 `PlanRun` / `startCase` 按 `order` 依次走同一个 plan
  的 case，跑完一个给 `Next case`，配置在 case 之间复用；建议的 run id 里编入 case
  （`ExecutionView.svelte` 的 `suggestRunId`）。
- `TestPlansView.svelte` 变成覆盖看板：每个 case 显示上次 run 的状态/操作员/日期/步数，
  外加一个 `Run next`（第一个不是 complete 的 case）。

### 2.2 计划用例定位的 bug 修复（"1 ERROR"的一个来源）

- 新增 `Repo::checklist_path(sop_id)`：先找 `checklists/<id>.md`，再在 `testplan/` 下按
  front matter 的 `sop_id` 找用例；`checklist_step_ids`、校验器、drift、`start_with_meta`
  全部走它。
- 之前从 plan 启动的 run 记录的是用例的 `sop_id`，仓库里没有同名 checklist 文件，于是
  校验器报 `'sop' … does not name an existing checklist`，整个仓库被判定为有错。

### 2.3 Phase 2 — 配置不再每次重敲

- 开始表单从"上一次 run"回填 operator / site / conditions / sensor（`seedFromRun`，只在
  操作员没输入时填）；回填的 sensor 会顺手写回 `sensors` 设置，下次能在菜单里选到。
- `project.md` 新增可选 `sites:`，与历史 run 的 site 一起作为开始表单的候选项
  （`ExecutionView` 的 `siteOptions`）。
- checklist 可声明 `conditions:`（三种写法：单个 key、key 列表、key→hint 映射），开始
  表单按声明渲染独立字段，而不是一整块自由文本；自由文本仍保留为兜底
  （`sop-core/src/front.rs` 的 `ConditionDecl` / `conditions()`，`check::checklist_conditions`
  负责校验，坏形状是 warning 不是 error）。
- `SensorsEditor.svelte` + `ui/src/lib/sensors.ts`：`sensors` 设置从裸字符串变成可增删的
  型号/序列号编辑器（存储形式仍是字符串，`formatSensors` 负责来回转换）。
- 开始表单实时校验：run id 冲突、必填项、draft 必须填 override，都在点 Start 之前显示。

### 2.4 Phase 3 — 信任信号与可追溯

- Header 徽章：校验错误/警告数（`Status.validation` 新增）、git dirty/ahead/behind；run 页
  常驻显示 `saved HH:MM:SS`。
- `CaptureAcknowledged` 事件 + 记录里的 `acknowledged:` 列表：超出 expected 范围但仍被
  接受的值留下痕迹，而不是只有一个会滚走的提示。
- `run_drift` 命令 + `run::Drift`：run 开始之后 checklist 被改过时，run 页显示
  "changed since this run started"（`ExecutionView` 的 drift effect）。
- `Reopen` 补偿事件：误点 done/skip/deviate 可以撤销。日志是追加式的，撤销也是新事件，
  不删记录。

### 2.5 note 编辑器（Markdown）

- 新增 `ui/src/components/MarkdownEditor.svelte`：6 行起步、可纵向拖动、占满整列的
  textarea + 实时 Markdown 预览；`Ctrl/Cmd+Enter` 保存，`Enter` 是换行。
- `ExecutionView.svelte`：单行 note 输入框换掉；新增 run 级 note（可折叠的 `Run notes`
  区块——`runNotes` 之前只有数据没有入口）；已保存的 note 以渲染后的 Markdown 显示在
  步骤下，不再"写了看不见"。
- `emit()` 现在返回是否写入成功，记录失败时不清空刚敲的内容。
- `crates/sop-core/src/run.rs`：多行 note 的渲染（step note 每行都带 `>`，run note 续行
  缩进 2 空格），各补一条测试。

### 2.5b 渲染器支持 `#`（h1）

- `ui/src/lib/markdown.ts` 原来只认 `#{2,4}`，所以 note 里写 `# test` 不会变成标题，还会
  被并进上一段；现在支持 `#{1,4}`，段落中断的正则同步改成 `#{1,4}\s`。
- **同一个渲染器有两份拷贝**：`ui/src/lib/markdown.ts` 和
  `crates/sop-cli/assets/preview.html` 内嵌的那段 JS。两份都要改，没有测试保证同步
  （见 §4 第 7 条）。
- `.prose h1` 补了字号（桌面 `ui/src/app.css` 18px，preview 20px），否则浏览器默认 2em
  会比页面标题还大。

### 2.7 Markdown 渲染移到 Rust（D22）

- 新增 `crates/sop-core/src/md.rs`：`render(source) -> String`，支持 `#`–`####`、表格、
  引用、列表（含任务项）、代码块、hr 与行内 code/strong/em/link；12 条单元测试，包含
  「操作员写的 `<script>` 必须被转义」。
- 窗口：新增 Tauri 命令 `render_markdown`，前端 `ui/src/lib/md.svelte.ts` 提供按源码文本
  缓存的 `htmlOf()`（模板里同步取，未命中时问一次 Rust，答案到了自动重渲染）。
  `ui/src/lib/markdown.ts` 删除，只留下 `text()` 移到 `ui/src/lib/text.ts`。
- `sop preview`：`preview.rs` 在服务端把每个 `body` 渲染成 `bodyHtml` 再发给页面；
  `preview.html` 里那 70 行 JS 渲染器整块删掉（只保留错误信息用的 `escapeHtml`）。
  已实测：fixture 仓库 2 个 checklist / 85 个 step 全部带上 `bodyHtml`，页面里已没有
  `function markdown`。
- 顺手修掉 JS 版的一个真 bug：有序列表 `<ol>` 用 `</ul>` 收尾。
- 依赖：`sop-core` 新增 `regex`（本来就在 workspace lock 里，离线可用）。

### 2.6 校验器：进行中的 run 不再报错

- 根因：`run_front_matter` 把 `status` 当无条件必填，但 `status` / `ended` 只在
  `sop run end` 时才写入。于是每次开跑，顶栏立刻显示 `1 ERROR(S)`。
- 现在：`status` 只在 run 结束后要求，且必须与 `ended` 成对出现（只有其一 = 手工改过，
  报错）；`validate.rs` 里 `run record has no step results` 只对**已结束**的 run 警告。
- 这是"进行中的记录合法"的语义澄清，已写进 `SPEC.md` §8 与 `SPEC-COMPAT.md` 变更表。

## 3. 文件清单（本批）

新增：`crates/sop-core/src/md.rs`、`ui/src/lib/md.svelte.ts`、`ui/src/lib/text.ts`、
`ui/src/components/MarkdownEditor.svelte`、`ui/src/components/SensorsEditor.svelte`、
`ui/src/lib/sensors.ts`、`docs/REVIEW-engineer-workflow.md`、本文件。

删除：`ui/src/lib/markdown.ts`（渲染器进 Rust，`text()` 移到 `text.ts`）。

修改：`ui/src/app.css`、`crates/sop-cli/assets/preview.html`、`SPEC.md`、`SPEC-COMPAT.md`、
`crates/sop-core/src/{check,front,run,vocab}.rs`、`crates/sop-repo/src/{lib,manifest,run,validate}.rs`、
`crates/sop-repo/tests/{run,negative}.rs`、`crates/sop-app/src/{api,commands,main}.rs`、
`ui/src/App.svelte`、`ui/src/components/{ExecutionView,Header,SettingsPanel,TestPlansView}.svelte`、
`ui/src/lib/{api,types}.ts`、`ui/dist/**`（重新构建的产物）。

**不属于本批**：`runs/**`（App 正在使用中产生的 run，见 §4 第 5 条）。

## 4. 待办 / 已知问题

1. **前端没在真实窗口里点过**（上一批也是这个问题）。建议走一遍：Test Plans → Run plan →
   某个 case 里写一条多行 Markdown note 和一条 run note → End → 在 History 导出里看
   note 的渲染。重点看 `MarkdownEditor` 的预览、note 落盘后的回显、以及开始表单的回填。
2. **`cargo fmt --all --check` 不通过**（先于本批存在）。成因是这些文件的行宽风格与
   rustfmt 默认值不同。**不要整体 reformat**，否则本批改动会被埋进几屏无关 diff 里。
   想在 CI 里加 fmt 检查，得先单独做一次纯格式化提交。
3. **`docs/FEATURES.md` 与 `docs/DECISIONS.md` 没更新**：Phase 1–3 的取舍（尤其是"进行中
   的 run 不是错误"、"note 是 Markdown 且可多行"）值得补 D20/D21；功能表里 note 编辑器、
   覆盖看板、drift 也应登记。
4. **`ui/dist/` 是被 git 跟踪的构建产物**，本批已重建（当前是 `index-Bbx888DM.js` +
   `index-BLIRRqbD.css`；每次 rebuild 文件名都会变，旧的删除）。提交时一起带上，否则窗口
   加载的仍是旧界面。
5. **不要把未跟踪的 `runs/` 顺手提交**：`runs/uas-mag-preflight-mount/`、
   `runs/uas-mag-preflight-power-on/`、`runs/heading-error/2026-09-28-heading-error-uas-mag*`
   都是 App 正在使用中生成的；`runs/heading-error/2026-09-28-cfar-01*` 的删除同样是 App 行为。
6. 本批作为**一个提交**落地（代码 + SPEC + docs + 重新构建的 `ui/dist`，未推送）。按主题
   拆成多个提交在这里没法干净切分：`ExecutionView.svelte`、`crates/sop-core/src/{run,check}.rs`
   与 `ui/dist/**` 同时装着 Phase 1–3 和 note/渲染器/校验器两类改动；把 `ui/dist` 单独提交
   的话，前一个提交 checkout 出来窗口加载的仍是旧界面。
7. ~~Markdown 渲染器有两份~~ 已在 §2.7 合并成 `sop_core::md` 一份（D22）。剩下的小事：
   窗口里 note 的实时预览是一次 IPC（本地、按文本缓存），如果哪天觉得卡，可以在
   `MarkdownEditor` 里做 100ms 防抖。

## 5. 怎么跑 / 怎么验

```bash
export PATH="$HOME/.cargo/bin:$PATH"

cargo test                              # 18 个目标 / 164 个测试
cargo check -p sop-app
cargo run -q -p sop-cli -- validate     # 0 error(s), 0 warning(s)

cd ui && npm run check && npm run build # 改了 ui/src/** 必须 build：窗口加载的是 ui/dist
REBUILD_UI=1 ./run-app.sh               # 重建前端并起窗口（隔离的 XDG 目录）
```

## 6. 路线图：Phase 4–7

`docs/REVIEW-engineer-workflow.md` 定义到 Phase 5，Phase 6/7 是**建议**，尚未立项。

- **Phase 4 — 数据采集与分析**：多文件/整目录 attach + 拖拽 + 单次 run 体积预算告警；按
  capture 的 `type` 渲染真正的输入控件；checklist 声明 `outputs:` 并逐个 tick-off，让
  `complete` 的含义变成"数据收齐"；同一 case 的多次 run 并排比较 + 整个 plan 的 export all。
- **Phase 5 — 平台打磨**：用 Tauri 原生对话框替换 `prompt` / `confirm`（skip/deviate 的
  原因、plan/case 管理）；把大附件拷贝移出 UI 线程；补 plan run 与配置持久化的测试。
- **Phase 6 — 交付与协作（建议）**：把 run / case / plan 导出成可交付的 PDF 或 HTML
  报告；run 交接（换人接着跑，operator 变更留痕而不是被覆盖）；只读分享视图；现场笔记本
  与办公室之间同一仓库双向推送时的冲突处理。
- **Phase 7 — 打包与运维（建议）**：签名安装包与自动更新、离线安装；大附件与 manifest
  重建等长任务改成后台任务 + 进度；定期备份/自动提交工作副本；中英界面切换；CI 补上
  `fmt`、`clippy` 和前端 e2e（tauri-driver 或 Playwright，覆盖 Run plan → note → export）。

## 7. 关键文件地图

| 位置 | 是什么 |
|---|---|
| `crates/sop-core/` | 格式：解析、校验、事件状态机、记录渲染（纯函数，无 IO） |
| `crates/sop-repo/` | 文件系统侧：发现、加载、include 解析、原子写、git、run 生命周期 |
| `crates/sop-cli/` | `sop` 命令 |
| `crates/sop-app/` | Tauri 壳：`commands.rs` 是前端能调的全部操作，`api.rs` 是线格式 |
| `ui/src/lib/api.ts` | 前端唯一的 IPC 出口，每个函数对应一个 Tauri 命令 |
| `ui/src/components/ExecutionView.svelte` | Run 视图（开始表单、步骤、capture、note、drift） |
| `ui/src/components/MarkdownEditor.svelte` | 可多行、带预览的 Markdown note 编辑器 |
| `ui/src/components/TestPlansView.svelte` | 计划覆盖看板与 Run plan 入口 |
| `ui/src/components/HistoryView.svelte` | History 视图（按 plan/case 分组；导出/汇总/推送/删除） |
| `docs/REVIEW-engineer-workflow.md` | 本批的评审结论与 Phase 1–5 路线图 |
| `SPEC.md` / `SPEC-COMPAT.md` | 格式权威说明 / 兼容性与变更表 |
