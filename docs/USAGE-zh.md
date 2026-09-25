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

顶部工具栏有四个视图按钮：**Run / Edit / Project / Settings**，以及两个开关 **Steps**（左栏步骤
显隐）和 **Help**（右侧帮助面板，F1 切换）。

### 1. Run 视图（运行/浏览）

默认视图，即操作员照着做的那一屏：

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

点工具栏 **Project** 进入。显示 `project.md` 的路径和每个字段，直接改值后点 Save。
写入同样先校验再落盘；改成非法值会被拒绝并报错。字段下方的说明文字解释了这个项目名
"是内容不是偏好"的设计。

### 4. Settings 视图（应用设置）

点工具栏 **Settings** 进入：

- **Working copy**：输入路径点 **Open** 打开另一个 field-sop 仓库；选择会被记住，
  下次启动落在上次所在仓库。
- **Git remote**：输入 URL 点 **Set remote**，URL 写进 git 配置，不进应用设置。
- **Application settings**：逐条列出 `repository / remote / branch / help-open /
  recent-repositories`，改值即存，带值的可点 Clear 恢复默认。
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