# CodeLab Studio

本地优先的交互式编程实践工作台，将实践手册与课程包转化为可运行、可验收、带上下文辅导的本地学习环境。

## Language

### 内容与工作区

**Course**:
只读的实践教学内容与验收定义源，可来源于内置课程、本地目录、单文件 Markdown 或 Git 仓库。
_Avoid_: Tutorial, Project, Repo

**Course Pack**:
采用标准目录结构分发的自包含 Course，在实践手册之外还提供课程元信息、Starter、验收规则以及可选的 Reference Solution。
_Avoid_: Package, Bundle, Template

**Starter**:
Course Pack 附带的初始代码模板（含透明可见的验收测试文件），仅在首次实例化空目录 Workspace 时复制，之后与学习者手写代码解耦。
_Avoid_: Boilerplate, Seed, Scaffold

**Workspace**:
绑定到某门 Course 的用户本地可写目录，承载学习者手写代码与独立的学习进度状态；同一门 Course 可实例化为多个 Workspace。
_Avoid_: Project, Course Directory, Sandbox

**Trial Workspace**:
课程创作期间用于隔离试跑草稿 Course Pack 的独立 Workspace，使创作者能够以学习者视角完整演练而不污染课程源文件。
_Avoid_: Preview Sandbox, Draft Folder

**Module Root**:
Workspace 内距离当前活跃文件最近的语言项目模块边界目录，用于界定可选语言服务与构建命令的作用域，以支持多模块仓库并存。
_Avoid_: Project Root, Package Dir

**Chapter**:
Course 内按顺序排列的一级学习单元，分为不强制验收的概念导读与计入完课进度的实战练习两类。
_Avoid_: Section, Lesson, Module

**Exercise**:
需要动手实践并计入课程完成度的 Chapter，具备结构化指令或验收条件时以三步引导流呈现；达成完成态后保持粘性，不因后续章节重构而自动回退。
_Avoid_: Problem, Task, Challenge

**Command Block**:
从实践手册解析出的可执行命令单元，包含工作目录与运行模式（单次或常驻），触发执行前必经结构化高危命令拦截检查。
_Avoid_: Code Block, Script, Snippet

**Reference Solution**:
隶属于某个 Exercise 的目标参考代码版本，仅用于与 Workspace 当前文件做差异对照，并在学习者显式确认后合并。
_Avoid_: Answer, Patch, RefPatch

### 运行时、执行、验收与信任

**Managed Runtime**:
由工作台按需隔离安装并管理的语言运行时与测试执行环境；仅在系统工具链缺失、版本不足或用户显式覆写时启用，不改动系统全局环境。
_Avoid_: Bundled SDK, Local Compiler

**Managed Daemon**:
由工作台启动并受其生命周期追踪的常驻服务进程；工作台感知其是否落后于最新保存的源码，且绝不强杀非工作台启动的外部进程。
_Avoid_: Background Job, Server Tab, Worker

**Check**:
隶属于某个 Exercise 的单条验收条件，包括可自动判定的客观校验与由学习者手动勾选的 Self-Check。
_Avoid_: Rule, Assertion (when referring to the full check item)

**Test Suite**:
运行 Workspace 内可见的测试文件的自动化 Check。通过要求真实测试框架产出可解析的有效报告、至少一个用例通过，且 Course 指定的必选用例全部通过。

**Command Test**:
根据命令退出码和 Course 声明的输出条件判定的自动化 Check。

**Self-Check**:
由学习者自主评估勾选的非自动化 Check，默认作为引导提示而不阻塞自动验收达成 Exercise 完成态（除非课程显式标为必选）。
_Avoid_: Manual Test, Checklist Note

**Check Result**:
单次验收执行为某个 Check 产出的实时判定快照（状态、耗时与预期/实际差异），与 Exercise 的历史完成态分离。
_Avoid_: Grade, Score, Test Output

**Trust State**:
外部或远程来源 Course 的安全准入状态；按来源标识持久化于工作区之外，在学习者显式确认信任前仅允许只读预览，禁止一切本地执行。
_Avoid_: Sandbox Mode, Approved Pack

### 助教与辅导

**Tutor Session**:
归属于单个 Workspace 的上下文辅导对话，统一承载消息流、Context Snapshot、教学风格以及本地 ACP 编码 Agent 的工具权限审批流。
_Avoid_: Chat Thread, Agent Process, LLM Session

**Context Snapshot**:
单轮助教提问中附带的可检视、可删减上下文集合（章节摘要、代码片段、终端报错尾与验收状态），受输入预算约束且优先保留用户选区与最新报错。
_Avoid_: Prompt Context, Memory, Attachment

**Permission Policy**:
约束本地 ACP 编码 Agent 访问能力的授权策略；默认放行工作区内非敏感只读，对文件写入、越界读取及命令执行强制弹窗审批。
_Avoid_: Sandbox Policy, Security Mode

### 桌面分发

**Desktop Release**:
CodeLab Studio 桌面应用的一个版本，包含相同版本号的各平台安装包，经维护者验收发布后向用户提供下载。
_Avoid_: Build

**Public Preview**:
面向早期用户公开下载的 Desktop Release，发布说明会注明适用平台和当前的安装限制。
_Avoid_: Release Draft
