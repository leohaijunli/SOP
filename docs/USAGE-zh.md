# field-sop 使用说明（中文）

field-sop 是一套管理"野外操作规程 + 检查清单 + 记录"的工具。内容全部是 Markdown
（格式见 `SPEC.md`），规则（校验、原子写、git）在 Rust 侧，界面只负责渲染。整套系统分
两层：

- **命令行 `sop`**：校验、索引、预览、查看/修改仓库与项目、查看/修改应用设置。
- **桌面应用 `field-sop`**：同一个渲染层，加上了运行视图、内容编辑视图、项目视图与设置视图。

本文用一个示例仓库（工作副本 `/tmp/fs-demo2`）把每个功能实际操作一遍。命令中的
`$SOP` 指代编译出的 `sop` 可执行文件，例如：

```bash
export PATH="$HOME/.cargo/bin:$PATH"
SOP=/home/dev/field-sop/target/debug/sop
```

先看这个仓库的内容：

```bash
cd /tmp/fs-demo2
```

---

## 一、命令行工具 `sop`

### 1. `sop status` —— 查看仓库整体状态

显示仓库身份、git 状态和内容统计。这是摸清"我在哪个仓库、里面有什么"最快的命令。

```bash
$SOP status
```

实际输出：

```
working copy   /tmp/fs-demo2
settings       /home/dev/.config/field-sop/settings.json
project        Geomagnetic Survey Field Work (uvic-geomag-survey)
institution    University of Victoria
git            not a working copy; sop reads and writes the files all the same
content        17 procedure(s), 2 checklist(s), 2 run record(s), 8 help page(s)
logs           2 run directories holding attachments
```

要点：

- `working copy`：当前仓库根目录。
- `project`：项目名 + 稳定 id，来自 `project.md`。
- `git`：如果不是 git 工作副本会明确提示，因为没有 git 也能读写文件。
- `content`：procedure / checklist / run / help 的数量。

> 演示：在目录之外运行会报"找不到内容"，说明你进错了目录。

```bash
cd /tmp && $SOP status   # 提示 no content / 找不到仓库
```

### 2. `sop validate` —— 校验内容是否符合规范

对全部（或指定）文件做格式校验。这是每次改完内容后必跑的命令。

```bash
$SOP validate
```

实际输出：

```
0 error(s), 0 warning(s)
```

- `0 error(s)` 是硬性要求：有错误意味着内容不合规。
- `warning` 是"奇怪但不一定错"，值得读但不阻断。

只校验某个文件：

```bash
$SOP validate checklists/ground-walk-survey.md
```

在别的目录校验本仓库：

```bash
$SOP validate --repo /tmp/fs-demo2
```

> 演示：往 `project.md` 写一个带空格的 `project_id`，`validate` 会拒绝并报错；
> 桌面应用/`project set` 也会在落盘前做同样检查（见下文）。

### 3. `sop index` —— 生成索引文件

把整个仓库合成一个文件 `dist/manifest.json`，桌面应用就靠它加载全部内容，不需要翻目录。

```bash
$SOP index
```

生成到默认位置 `dist/manifest.json`：

```bash
$SOP index --out /tmp/my-manifest.json   # 指定输出位置
```

> 桌面应用每次刷新/命令都会现场重新生成 manifest，所以改了 Markdown 无需重新编译。

### 4. `sop preview` —— 浏览器里预览布局

不用启动桌面应用，直接在浏览器里看渲染效果（左栏步骤、中间指令、右侧帮助面板）。

```bash
$SOP preview --port 8731
```

打开 http://127.0.0.1:8731/ 即可看到。

- 只读：只用于确认布局和内容。
- 支持深链：`#<sop_id>/<步骤id>/<帮助id>`。
- 改完 Markdown 刷新页面即可生效（manifest 每次请求现生成）。

### 5. `sop remote` —— 查看 / 设置 git 远端 URL

远端 URL 由 git 保管，**不会**写进应用设置，因此拷贝仓库不会带走任何 token。

查看当前远端：

```bash
$SOP remote
```

设置远端：

```bash
$SOP remote git@github.com:org/field-sop.git
```

> 安全提示：如果 URL 里带着 `https://user:token@...`，`sop remote` 会成功但会警告你
> URL 里有凭据。而 `sop settings set remote <url>` 会被**拒绝**——远程 URL 不属于设置。

### 6. `sop project` —— 查看 / 修改项目身份

项目名是**仓库内容**（写在 `project.md`），不是应用偏好。这样两个操作员看同一份记录永远
看到同一个名字，而且这个名字会随记录一起被 review。

查看项目字段：

```bash
$SOP project
```

实际输出（节选）：

```
project.md
  title          Geomagnetic Survey Field Work    The name the app shows...
  project_id     uvic-geomag-survey               Stable id. Changing it is a rename...
  institution    University of Victoria           Who is responsible for the work.
  ...
```

修改某个字段，只改那一行，文件其余部分（正文、注释、键顺序、未知键）逐字节保留：

```bash
$SOP project set title "Renfrew 2026"
$SOP project set updated "2026-09-25"
```

演示安全行为：

```bash
$SOP project set project_id "No Spaces"   # 拒绝：id 不允许空格，文件一字不动
```

写入是"先校验再落盘"的原子写，校验不过就拒绝，绝不留一个连自己都校验不过的文件。

### 7. `sop settings` —— 查看 / 修改应用自身设置

设置是应用自己的偏好（打开哪个工作副本、用哪个 git remote、帮助面板是否默认打开），
**刻意不放进仓库**——一条会被提交的偏好就是一条会冲突的偏好。

查看设置：

```bash
$SOP settings
```

实际输出：

```
settings   /home/dev/.config/field-sop/settings.json
           no file yet; these are the defaults
  repository           (unset)                      Working copy to open. A path, or a path to create.
  remote               origin                       Name of the git remote to use, for example origin. Not a URL.
  branch               (unset)                      Branch runs are recorded on. Absent means the current branch.
  help-open            true                         Whether the help panel starts open: true or false.
  sensors              UAS-MAG; RM3100              Sensor models and their serial numbers, as model: serial, serial; model.
  devices              (unset)                      Other equipment, as kind/name: serial, serial; kind/name.
  recent-repositories                               Working copies opened before, most recent first.
```

修改一个设置：

```bash
$SOP settings set repository /tmp/fs-demo2
$SOP settings set help-open false
```

恢复默认：

```bash
$SOP settings unset repository
$SOP settings unset help-open
```

演示安全行为（把 URL 当 remote 名）：

```bash
$SOP settings set remote https://github.com/org/repo.git
# 拒绝：remote 是名字不是位置，会提示 URL 请用 `sop remote` 写进 git 配置
```

注意：`settings` 里**没有任何键保存凭据或 URL**。URL 在 `.git/config`，由 git 保管。

---

## 二、桌面应用 `field-sop`

启动（在演示仓库目录内，用隔离的配置以免碰到真实设置）：

```bash
cd /tmp/fs-demo2
export XDG_CONFIG_HOME=/tmp/fs-cfg2
field-sop &
```

窗口启动后，顶部是项目名（从 `project.md` 读的，不是写死的），后面跟着内容统计。

### 视图切换

窗口**默认落在 Test Plans**（早上先挑今天要跑什么，而不是先改内容）。顶部工具栏顺序是
**Test Plans / Run / History / Browse**；改内容、改项目、改设置收在右侧的 **Manage ▾**
下拉里（**Edit SOP / Project / Settings**）——现场误触一下不会改到 SOP。

Browse 视图下还有两个开关 **Steps**（左栏步骤显隐）和 **Help**（右侧帮助面板，F1 切换）。

### 1. Run 视图（运行/浏览）

从 Test Plans 点 Run，或者从计划里的 Start / Run next 进入，即操作员照着做的那一屏：

- **左栏**：选 checklist（下拉）+ 过滤框 + 步骤列表。每个步骤前的圆点颜色是 severity
  （info 蓝 / normal 亮 / critical 红）。
- **中栏**：当前步骤完整指令——captures 字段、勾选项、以及从 procedure 里 `include`
  进来的原文。`expected` 只做高亮提示，不自动判合格，判不判由操作员决定。
- **右栏**：帮助面板，按 section 分组（Getting started / Using the app / Commands /
  Authoring / Troubleshooting / Reference）。

键盘快捷键（与 preview 一致）：

| 键 | 作用 |
|----|------|
| `j` / `k` | 上 / 下切换步骤 |
| `F1` | 开关帮助面板 |
| `/` | 打开帮助搜索 |
| `?` | 跳到快捷键说明帮助页 |

地址栏带 `#<sop_id>/<步骤id>/<帮助id>` 可深链到某一步或某页帮助。

### 2. Edit 视图（内容编辑 / Authoring）

点工具栏 **Edit** 进入。这一屏编辑当前 checklist 的内容，写回走的是
`sop-repo` 的"先校验再原子写"——校验不过就拒绝、文件不动。

- **Procedures to include**：列出所有可 include 的 procedure，勾选即把它的步骤展开到
  本 checklist 的 include 标记处。
- **Add a step**：新增步骤——填 id、标题、kind（check/measure/select/note/gate）、
  severity（info/normal/critical）、正文（Markdown），保存。
- **Steps in this checklist**：对每个已有步骤可
  - 改标题 / kind / severity / 正文；
  - 用 ↑ ↓ 调整顺序；
  - 用 × 删除（会二次确认）；
  - 增删 captures（key / label / type / required）。

演示：在 Add a step 里填 `id: line-check`、`title: Line check`、`kind: check`，点 **Add
step**。然后保存 → 切回 Run 视图，左栏就能看到新步骤。改完 Markdown 后跑
`sop validate` 校验一遍。

### 3. Project 视图（项目身份）

点工具栏 **Manage ▾ → Project** 进入。显示 `project.md` 的路径和每个字段，直接改值后点 Save。
写入同样先校验再落盘；改成非法值会被拒绝并报错。字段下方的说明文字解释了这个项目名
"是内容不是偏好"的设计。

### 4. Settings 视图（应用设置）

点工具栏 **Manage ▾ → Settings** 进入：

- **Working copy**：输入路径点 **Open** 打开另一个 field-sop 仓库；选择会被记住，
  下次启动落在上次所在仓库。
- **Git remote**：输入 URL 点 **Set remote**，URL 写进 git 配置，不进应用设置。
- **Application settings**：逐条列出 `repository / remote / branch / help-open / sensors /
  devices / recent-repositories`，改值即存，带值的可点 Clear 恢复默认。`sensors` 是
  Start run 面板里那份仪器清单（`型号: 序列号, 序列号; 型号`）；`devices` 是仪器之外的
  其他设备（GNSS 接收机、基站、无人机、电池、地面站、电台……），写法是
  `kind/name: 序列号, 序列号; kind/name`，`kind/` 可省略。设备编辑框按 kind 分组，kind
  输入框带常用值下拉，也可自填；kind、名称、序列号里不能出现 `; : , /`。
- 每次操作会在下方显示成功或报错消息。

---

## 三、怎么往内容里加东西（内容写作）

步骤不是写死的，全部来自 Markdown。举例——在 `checklists/ground-walk-survey.md` 里加
一步，可以写成：

```markdown
## Compare against a reference

```yaml step
id: abs-reference
kind: measure
severity: normal
captures:
  - key: reference_value_nt
    label: Reference value
    type: number
    unit: nT
    required: true
```

Take the reference reading twice, at least five minutes apart.
```

步骤能定义的东西：

- `id`：稳定标识，记录会引用它，永不改、不复用。
- `kind`：check / measure / select / note / gate；`severity`：info / normal / critical。
- `captures`：操作员要填的字段，每个有 key、label、type
  （text/number/integer/bool/select/datetime/duration/attach）、unit、required、
  options、expected。
- 正文任意 Markdown（表格、警告、判断标准），以及 `- [ ]` 勾选项。

共享步骤用 include 复用：

```markdown
<!-- include: procedures/power-on.md -->
```

`include` 标记在哪一行，被展开的块就出现在哪一行；顺序 = 文件里的书写顺序，想调顺序就
把段落挪位置。

改完如何生效：**存盘 → 刷新页面 / 重新调用 → `sop validate`**（桌面应用与 preview 每次
请求都现场生成 manifest，无需重新编译）。

---

## 四、排查

- `sop validate` 报 `no content found`：你在错误目录，进仓库根目录或加 `--repo`。
- 深链指错步骤：现在的解析优先按 id（id 才稳定），序号也认。
- 改了内容页面不刷新：preview / 桌面应用 manifest 是现生成的，强刷页面即可。
- 落盘被拒：改动会让仓库不再 validate，工具会拒绝并保持文件不变——把改动改到合规再存。

---

## 五、部署到另一台 Ubuntu 机器

**铁律（`DESIGN.md` §7）：不要在 Arch 开发机上构建发布版再拷过去。** 本机 glibc
是 2.44，Ubuntu 24.04 是 2.39、22.04 是 2.35，二进制在 Ubuntu 上起不来。发布产物必须
在 Ubuntu（或目标版本的容器 / CI）里构建。

### 方案一（推荐）：CI 构建 `.deb`，目标机一键安装

1. 给 CI 加打包 job（用 `ubuntu-22.04`，22.04 的包在 24.04 也能装）：
   - 装系统依赖：`libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libsoup-3.0-dev
     libayatana-appindicator3-dev build-essential`
   - 装 Rust + Node，`npm ci && npm run build`，再 `cargo install tauri-cli` 并
     `tauri build --bundles deb`
   - 用 `actions/upload-artifact` 上传 `target/release/bundle/deb/*.deb`
2. 把 `tauri.conf.json` 的 `bundle.active` 改为 `true`，补全各尺寸图标和 `.deb` 的
   `depends`（`libwebkit2gtk-4.1-0`）。
3. 目标机部署（连网）：
   ```bash
   sudo apt install ./field-sop_0.1.0_amd64.deb
   ```
   `.deb` 声明了 `webkit2gtk`，装它时自动拉进渲染引擎，菜单里直接有入口。

### 方案二：在目标 Ubuntu 机器上直接源码构建

```bash
sudo apt install build-essential libwebkit2gtk-4.1-dev libgtk-3-dev \
                 librsvg2-dev libsoup-3.0-dev libayatana-appindicator3-dev curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # 然后装 Node 18+
git clone git@github.com:leohaijunli/SOP.git && cd SOP
cd ui && npm install && npm run build && cd ..
cargo run -p sop-app        # 或 ./run-app.sh
```

### 方案三：目标版本容器里打 AppImage（单文件，免安装）

容器内 `tauri build --bundles appimage`，得到一个单个可执行文件拷到目标机。但
**AppImage 不声明依赖**，目标机必须已装 `libwebkit2gtk-4.1-0`，否则起不来——所以
`.deb` 才是主产物，AppImage 只是便利。

---

## 六、location（地点）是什么、如何设置/编辑

系统里"地点"不是单一设置，出现在两处，编辑方式不同。

### 1. 步骤上的 capture 字段 `session_location`（问题本身）

定义在 `procedures/conditions-log.md` 的 `yaml step` 块里：

```markdown
## Location and scene

```yaml step
id: cond-location
kind: check
severity: normal
captures:
  - key: session_location      # ← 字段名
    label: Where the session took place   # ← 界面上显示的问法
    type: text
    required: true
```

- **改"问法 / 字段名 / 是否必填"**：改这个 `yaml step` 块（桌面应用里是 **Edit** 视图
  的 captures 编辑，或直接改 Markdown）。
- **它本身不是"值"**：它是一道题，真正的值在每次运行时填。

### 2. 值在运行记录（run）里——这就是你现在编辑不了的

你在界面看到的是 `session_location` 这道题，桌面应用当前**没有记录模式**（DESIGN 的
P3/P4 阶段，还没写），所以它老实显示 "recording arrives with the app"——不是占位符，
是实话。值要写进**当次运行的文件** `runs/<sop_id>/<run_id>.md`，放在与该步骤对应的
`yaml result` 块里：

```markdown
## Location and scene

```yaml result
step: cond-location
status: done
captures:
  session_location: "Renfrew 395, hill top near the water tank"
  nearby_sources: "Power line 200 m east"
```
```

`step` 必须对上该步骤的 id（`cond-location`）；`status` 为 `skipped` / `deviated` 时还
要带 `reason`。每个执行过的步骤各写一个 `yaml result` 块。整个运行的地点写在 run 的
front-matter `site` 字段（如 `site: Renfrew 395`）。

> 结论：capture 的**定义**用 Edit 视图改；capture 的**值**和整个运行的 **site** 属于
> 运行记录。现在**记录模式已实现**（`sop run` / 桌面应用 Run 视图）——开跑后把这些
> 值填进表单，每填一项就是一条事件，自动写进 `runs/<sop>/<run>/events.jsonl` 并渲染
> 成 `record.md`。改值用 `CaptureCleared` 事件带原因，绝不覆盖旧值。

---

## 七、运行一条记录（run mode）

一条 run 的真相源是事件日志 `events.jsonl`；每一步操作都是一条 typed event，只追加
不覆盖。崩溃后重放日志即可恢复到最近一致状态。

### 命令行

```bash
# 开跑（冻结 snapshot、开 events.jsonl、写初始 record）
sop run start <sop> <run_id> <operator> <site> [--override REASON]

# 记录事件
sop run record <sop> <run> capture  <step> <key> <value> [--unit UNIT]
sop run record <sop> <run> checkbox <step> <index> true
sop run record <sop> <run> done     <step>
sop run record <sop> <run> skip     <step> <reason>     # reason 必填
sop run record <sop> <run> deviate  <step> <reason>     # reason 必填
sop run record <sop> <run> note     [--step STEP] <text>

# 恢复（重放日志）
sop run recover <sop> <run>
# 拷入附件（拷贝+哈希+文件名；--kind 选 logs/photos/attachments，默认 log）
sop run attach <sop> <run> [--step STEP] [--kind log|photo|file] <file>
# 结束并写出人类可读的 run 记录文件 runs/<sop>/<run>.md
sop run end <sop> <run> complete|partial|aborted
# 打包一次汇总：summary.md / summary.csv + 每个 run 的完整文件夹
sop export --out <目标目录> [--label 标签]
```

安全行为演示：

```bash
sop run start ground-walk-survey ...            # 拒绝：draft 清单必须 --override 给原因
sop run record ... skip cond-location           # 拒绝：skipped/deviated 必须带 reason
```

### 桌面应用 Run 视图

点工具栏 **Run** 进入。选清单 → 填 operator / site，从下拉列表里选 sensor 型号和序列号
（型号来自 `sensors` 设置，默认 `UAS-MAG`、`RM3100`；手输的新序列号会自动加进列表，
清单 `equipment:` 里声明且与型号同名的仪器会被自动选中）→ 核对 checklist 声明的设备
勾选，再在 **Equipment used** 分组里勾选 `devices` 设置中的其他设备（有多个序列号的
给下拉；默认全选，没用到就取消）→ run id 已按日期+型号+序列号自动生成
（`2026-09-28-uas-mag-1001`，同名再加 `-2`），想改就点 **Change**；draft 还要 override
原因 → Start。之后每一步可：填 captures（有单位、期望范围高亮，越界会提示"acknowledge"）、
勾选 checkboxes、Mark done / Skip / Deviate（后两者弹原因）、加备注、Attach 附件；
右侧 End 按钮用 complete/partial/aborted 结束。崩溃重开会自动重放恢复未结束的 run。

### 桌面应用 History 视图

点工具栏 **History** 进入。run 按它启动时的 test plan / case 分组；每组一张表，所有表
共用同一套列宽（`run id / started / site / sensor / operator / outcome / conclusion /
dev / files / ▸`），所以不同 case 的列左边界对齐，窗口最窄也不破版。`files` 列显示
`日志·照片·文件` 计数，已结束但还没传日志的 run 会标出来。

每一行点开详情抽屉：

- **Add log… / Add photos… / Add files…**：各自多选，按 kind 落到 run 目录下的
  `logs/`、`photos/`、`attachments/`；单张超过 50 MB 会先确认。批量逐个处理，单个失败
  不影响其他，最后汇总提示。run 已经结束也能补传——白名单事件（附件、备注、结论）允许
  在封存之后追加，封存本身不移动（`docs/DECISIONS.md` D29）。
- **Add note…**：run 级备注，进 `record.md` 和生成的 `notes.md`。
- 文件/备注列表：类型、名称、大小、时间，以及 `after run` 标记（区分现场记录和事后补充）。
- **Open folder**：在桌面文件管理器里打开这个 run 的目录（只用 id 拼路径，不接受任意路径；
  失败时显示路径）。
- **Export record**：把该 run 的记录写到 `exports/<sop_id>-<run_id>.md`。
- **Delete**：删除该 run 的记录、run 目录和全部附件（先确认，不可撤销）。

工具栏的 **Export summary** 不是只导一张表，而是把一份报告和所有 run 打包成一个自包含
目录 `<目标目录>/export_<本地时间>/`：`summary.md`、`summary.csv`、每个 run 的正式记录和
完整 run 文件夹，外加 `MANIFEST.sha256`。目录选择器默认落在 `exports/`（已被 git 忽略）；
完成后提示「导出了 x 个 run / y 个文件 / z MB → 路径」，并给出 **Open export folder**。
表格列与 History 一致；中途失败不留半成品；同名目录自动加 `-2`
（`docs/DECISIONS.md` D30）。

### 知识闭环（P5）

```bash
# 跨 run 的偏差汇总：哪一步总在出问题
sop run deviations <sop>
# 导出为 CSV 给处理管线
sop run export <sop> --format csv
```

run 记录文件 `runs/<sop>/<run>.md` 是带 front-matter 的合法 run 文件（run_id、sop、
sop_version、sop_commit、started/ended、status、deviations_count），可被 `validate`、
`manifest`、桌面应用 History 视图读取，随记录一起提交进 git。
---

## 八、教程：一份 md 文档 → app 里自动出现的 steps 和记录

目标：你只写 Markdown，app 自动把它变成可执行的步骤、勾选项和实验记录。

### 1. 全部机制只有三条规则

| 你写的东西 | app 当成什么 |
|---|---|
| 文件放在 `procedures/` | 可复用的步骤组，供 checklist 用 include 引用 |
| 文件放在 `checklists/` | 一次实验的清单，就是 app 里可选的那个 SOP |
| `## 章节` | **一个 step**（它下面的 `yaml step` 块给出这个 step 的身份） |
| `### 子章节` | 留在所属 step 的正文里，不会变成新 step |
| ```yaml step``` 块 | 这一步的 `id` / `kind` / `severity` / `captures` |
| `- [ ]` 行 | 勾选项；文字就是操作员在 app 里看到并勾的那句 |
| `<!-- include: procedures/x.md -->` | 在这一行的位置展开 x 的步骤 |

**顺序 = 书写顺序**：`##` 出现的先后就是执行顺序，include 标记在哪一行，被展开的块就落在哪一行。没有 `order` 字段，也不需要。

**每个 `##` 章节都必须有 `yaml step` 块**，哪怕只有两行。没有块 = 那一章只是散文，`sop validate` 会点名报错。原因见 §4 的 id 说明。

### 2. 完整示例：一份 procedure

`procedures/zero-drift-check.md`：

````markdown
---
kind: procedure
procedure_id: zero-drift-check
title: Zero Drift Check
version: 1
updated: 2026-09-26
applies_to: [ground-survey]
tags: [calibration]
---

A magnetometer that drifts through the day turns a real gradient into a slow ramp.

## Warm-up baseline

```yaml step
id: drift-baseline
kind: measure
severity: critical
captures:
  - key: baseline_nt
    label: Baseline total field
    type: number
    unit: nT
    required: true
  - key: settle_min
    label: Minutes since power-on
    type: integer
    required: true
```

### Before you start

Power the sensor and leave it alone for at least twenty minutes.

- [ ] Sensor powered for at least 20 minutes
- [ ] Operator and stand checked for ferrous items
- [ ] Instrument not moved since power-on

## Re-check after the first line

```yaml step
id: drift-recheck
kind: measure
severity: normal
```

Repeat the reading at the same spot. A difference larger than 5 nT means the sensor is
drifting and the line has to be repeated.
````

注意 `### Before you start` 是子章节：它和后面的 `- [ ]` 都属于 `drift-baseline` 这一步，
不会各自变成 step。

### 3. 完整示例：一份 checklist

`checklists/drift-survey.md`：

````markdown
---
kind: checklist
sop_id: drift-survey
title: Drift Check Survey
version: 1
updated: 2026-09-26
status: draft
applies_to: [ground-survey]
equipment:
  - Total-field magnetometer
  - Non-magnetic stand
---

A short session that runs the drift check, walks one line, and closes out.

## Session setup

```yaml step
id: drift-setup
kind: check
severity: normal
```

Write down where this is. The baseline only means something next to a location.

- [ ] Site and date recorded
- [ ] Base station logging

<!-- include: procedures/zero-drift-check.md -->

## Session close-out

```yaml step
id: drift-closeout
kind: note
```

Note anything that would change how a reader interprets the baseline.
````

`status: draft` 表示还没有用于正式数据；运行它要在 app 里给一个 override 原因（这点设计是故意的）。

### 4. app 自动生成的结果

```bash
sop validate      # 0 error(s), 0 warning(s)
sop index         # 重建 dist/manifest.json，app 每次打开现场重新生成，其实不必手动跑
```

`drift-survey` 这份清单被解析成 4 个 step，顺序就是章节顺序：

| # | id | 标题 | kind | severity | 来自 |
|---|---|---|---|---|---|
| 1 | `drift-setup` | Session setup | check | normal | `checklists/drift-survey.md` |
| 2 | `drift-baseline` | Warm-up baseline | measure | critical | `procedures/zero-drift-check.md` |
| 3 | `drift-recheck` | Re-check after the first line | measure | normal | `procedures/zero-drift-check.md` |
| 4 | `drift-closeout` | Session close-out | note | normal | `checklists/drift-survey.md` |

这就是 "id 用于内部引用" 的实现：记录里只写 id，app 用它把 step 和记录对上。

app 里对应三处：

- **Browse**：左边是这 4 步，右边是标题、正文（含子章节）、captures、`- [ ]` 条目。
- **Run**：左边 step 列表带状态颜色（open/done/skipped/deviated）；右边是 badges、
  正文、Captures 输入框、**Checklist 勾选框（显示你写的那句文字，不是 "Item 1"）**、
  Mark done / Skip / Deviate、**Note for this step**。
- **Edit**：一次编辑一个 step（左列表右表单）；写回的是定义该 step 的那个文件。

### 5. 每步的 note 去哪了

**Note for this step** 写进记录，作为 `> note:` 行；勾选状态写进 `- [x]`；结果写进
```yaml result``` 块。真实输出（`runs/drift-survey/2026-09-26-drift-02.md`）：

````markdown
## Warm-up baseline

### Before you start

Power the sensor and leave it alone for at least twenty minutes.

- [x] Sensor powered for at least 20 minutes
- [ ] Operator and stand checked for ferrous items
- [ ] Instrument not moved since power-on

_(not completed)_

> note: wind picked up, the stand rocked once

## Re-check after the first line

Repeat the reading at the same spot. A difference larger than 5 nT means the sensor is
drifting and the line has to be repeated.

```yaml result
step: drift-recheck
status: done
```
````

**给每一步一个结果**（Mark done / Skip / Deviate）仍然是推荐做法：`complete` 的记录要
求每一步都有结果，缺结果会校验失败。但**没有结果也不再等于丢数据**：只要这一步填过
capture，记录里就会出现一个 `yaml result` 块，带数据、不带 `status` 行——表示"做了、
有数据、没下结论"。备注和附件同样保留。没有结果、也没有任何数据的 step 才不会写块。

### 6. 常见报错

| 报错 | 原因 | 改法 |
|---|---|---|
| `chapter 'X' has no yaml step block` | 那一章没有块，所以不是 step | 加 ```yaml step``` 块，最少写 `id` 和 `kind` |
| `step 'id' must be a string` | 写了 `id: 7`，YAML 把裸数字读成整数 | 写成 `id: "7"` |
| `step id '1' ... collides with ...` | 数字 id 在 include 时必然撞车 | 见下一节 |
| `duplicate step id 'x'` | 同一个文件里 id 重复 | 换一个 id |
| `pre-checked item '- [x]' in a template` | 模板里预勾选了 | 模板一律写 `- [ ]` |

### 7. 关于 id：为什么不能用 1、2、3

`id: "1"` 语法上是合法的，但**在 include 会立刻撞车**：id 的唯一性是在整个清单解析
之后判定的，而每个 checklist 都有自己的本地步骤、每个 procedure 又会从 1 重新开始。
实测：

```
error: line 16: step id '1' from procedures/zero-drift-check.md collides with
checklists/drift-survey.md; included procedures must not share step ids
```

还有第二个代价：如果 id 是"按位置递增"，在中间插入一章就得重排后面的编号；而 id 是
**记录的引用键**，一改就断——历史记录本身不会失效（它读的是自己的 snapshot），但
`sop run deviations` 这类跨 run 的统计会把同一个 step 拆成两个桶。

推荐写法：**procedure 缩写 + 序号**。短、唯一、插章节也不用重排：

```yaml
id: zdc-1     # zero-drift-check 第 1 步
```

## 九、运行记录里还有什么：sensor / hardware / conditions

Start run 面板里除了 run id / operator / site，还能填三项**可选**信息，它们会写进记录
（front matter + 正文开头各一份）。sensor 的型号和序列号从下拉列表选（列表来自机器设置
`sensors`），`hardware` 默认就是清单 `equipment:` 里勾中的那几项：

```yaml
sensor:
  model: GEM GSM-19
  serial: "4451233"
  firmware: "7.0"
hardware:
  - mag_gcs v0.3.1
  - tripod
conditions:
  weather: clear
  temp_c: 12
```

- 三项都可以留空；留空就不写对应的块。
- `serial`、`temp_c` 这类看起来像数字的值会被加引号，否则 YAML 会把它们读成整数，
  序列号里的前导零就没了。手写在文件里的裸数字也照样合法。
- `hardware` 一行一项；`conditions` 一行一个 `key: value`。新增维度（比如 `humidity`）
  不用改代码。
- 为什么要重复一遍：front matter 给机器读，正文开头给"只打开导出文件的人"读，两个 run
  放一起对比时一眼能分清是同一台仪器还是换过固件。

### 键盘操作（Run 视图）

| 键 | 作用 |
|---|---|
| `F1` | 显示 / 隐藏帮助面板 |
| `j` / `k` | 下一步 / 上一步 |
| `space` | 当前步标记 `done` |
| `s` | 标记 `skipped`（会问原因） |
| `d` | 标记 `deviated`（会问原因） |
| `n` | 跳到当前步的备注输入框 |
| `ctrl` + `enter` | 请求结束 run（`complete`）；会出现确认条，再点一次才真正结束 |
| `/` | 打开帮助面板 |

在输入框里时，`ctrl` + `enter` **不会**结束 run：备注编辑器里它是"保存这条备注"，
其它输入框里它只是普通按键。想结束 run，先把光标移出输入框（点一下空白处，或按 `Esc`）。
其它单键快捷键会被输入框吃掉，这是故意的——光标在输入框里时敲 `s` 应该是在写字，不是在
跳过步骤。结束 run 不可撤销，所以 `ctrl` + `enter` 只是先把确认条调出来，`Esc` 可以取消。

真正结束 run 时，会先问这次运行的**结论**：`pass` / `fail` / `inconclusive` 三选一点一下，
再写一句原因（这句会写进记录，不能留空）。结论和 `status`（`complete` / `partial` /
`aborted`）是两回事——`complete` 只表示"跑完了"，结论是人给的判定，工具从不替人判
（D4）。如果不想记结论，直接跳过这两问，run 一样照常结束，和以前行为一致。

`space` 标记 `done` 时，如果当前步还有 `required` 的 capture 没填，会先提示还有几项为空，
并让你写一句原因；选"取消"则不标记。这条提示只是提醒，不影响数据是否合格（合格与否由人判断，见 D4）。

### 录入时的导航（Batch 2）

- **进度**：运行页顶部显示 `N / M done`，进度条里 `done` 和 `deviated` 都算"有结果"。
- **自动前进**：标记某一步 `done` 后，光标会自动跳到下一个还没有结果的步骤（跑完最后一步
  会绕回第一个还没做的），不用自己找。
- **自动聚焦**：切到某一步时，光标落在这一步第一个还没填的 capture 上。
- **结束 run 被挡住时**：`End: complete` 置灰，左侧会写明"还有 N 步挡着"，并给一个
  **Go to &lt;步骤名&gt;** 按钮直接跳过去。`done` 或 `deviated` 的步骤都不算挡路，`skipped` 算。
- **没做完的 run**：开始表单顶部列出整个仓库里所有还没结束的 run（跨 checklist）。
  点 **Resume** 就接着做；如果那条 run 属于别的 checklist，界面会先切过去。
- **原因输入**：Skip / Deviate / Acknowledge / 空必填项提醒都弹应用内的对话框，常用理由
  是一个按钮点一下，也可以在文本框里自己写；`Enter` 提交，`Shift+Enter` 换行，`Esc` 取消。
- **开始表单折叠（Batch 2）**：表单上只留必填的 **run id / operator / site**；
  仪器、设备、条件和附加硬件收在 **Same as last run: …** 一行里——从这次 checklist 的
  上一次 run 回填，一行摘要写明沿用了什么。没有历史可沿用（或换了 checklist）时这行会
  自动展开，标题变成 "Instrument, equipment, and conditions"。要改就点开它。
