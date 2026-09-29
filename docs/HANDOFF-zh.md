# 交接说明（中文）

本文件记录**刚提交的这一批**改动、验证到什么程度、以及下一步怎么走。格式的权威
说明是 `SPEC.md`，设计取舍编号索引是 `docs/DECISIONS.md`（本批补到 D21），功能优先级表
是 `docs/FEATURES.md`，本批的评审与路线图是 `docs/REVIEW-engineer-workflow.md`（Phase 1–5，
Phase 6/7 为建议）。中文操作教程在 `docs/USAGE-zh.md`。

> 本批两个提交，都在本地 `main` 上、**尚未推送**：先是 Phase 1–3 + note 编辑器（`4766953`），
> 然后是 Markdown 渲染器 Rust 化（见 §2.7）。工作副本里剩下的未提交内容只有 `runs/` 下
> App 正在生成/删除的 run。上一版交接（D15–D19 那一批）同样在历史里：
> `git show 94dae4d:docs/HANDOFF-zh.md`。

## 1. 当前状态

```
cargo test                  15 个测试目标 / 145 个测试全部通过
cargo check -p sop-app      通过
sop validate                0 error(s), 0 warning(s)；若 App 里有一条已结束但没记任何结果的
                            run，会多一条 "no step results" warning，这是预期行为
cargo test                  新增 md::tests（12 条）后仍然全绿
cd ui && npm run check      0 errors, 0 warnings
cd ui && npm run build      通过（ui/dist 已重建，且被 git 跟踪）
git                         本批已提交到本地 main（未推送）
```

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

cargo test                              # 15 个目标 / 145 个测试
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
