<!-- Release-notes skeleton. Entries accumulate in ./Changelog.md as PRs land;
     before publishing a release, distill them:
     - user-visible changes only — no dev tooling, CI fixes, or refactors whose
       effect is already covered by another entry
     - describe outcomes, not internals: no internal module names, IPC or
       protocol details, or implementation mechanics — write what the user
       observes, not how the code achieves it
     - one line per theme: merge entries that share a root cause
     - each entry fits a single readable line (about 60 CJK characters); if it
       overflows, it is packing multiple themes or detail that belongs in the
       linked issue/PR — split it or drop the detail
     - line grammar: open with a verb (修复/新增/优化), with the platform
       first (Windows/macOS/Linux) when the change is platform-specific;
       merge sub-symptoms as 主题：症状、症状; full-width punctuation,
       「」 for UI labels, backticks for config keys and commands
     - platform entries sit under their platform heading (bold, emoji
       prefixed, ordered Windows/macOS/Linux, combined for pairs like
       macOS/Linux) at the end of their section;
       scripts-workflow/group_platforms.sh regroups, --check verifies
     HTML comments are stripped by the update dialog's sanitizer and hidden on
     GitHub, so they never reach users. -->
## v2.6.1

<details>
<summary><strong> 🐞 修复问题 </strong></summary>

- 修复应用退出时无条件重写 `clash.yaml`、`verge.yaml` 和 `profiles.yaml` 导致文件创建时间丢失的问题
- 修复无法导入和更新 Base64 编码的 VLESS、Hysteria2 节点订阅的问题
- 修复规则扩展包含空列表项时，「编辑规则」界面报错的问题
- 修复托盘测速与代理页行为不一致的问题，移除托盘「测试延迟」入口
- 修复通过 URL 导入订阅时，即使导入的订阅并非当前使用的订阅，也会重新加载当前配置
- 修复 首页 网站测速失败后卡片一直显示测试中
- 修复 代理 页展开分组后不显示最新测速结果，需切换节点才刷新
- 修复 代理 在全局模式下无法展开节点组
- 修复 自定义 CSS 无法修改编辑器字体和字号的问题
- 修复检测更新后又发布新版时，应用内更新失败
- 修复订阅响应过大导致内存占用过高，限制单次下载为 4 MiB
- 修复全局模式默认停在 `DIRECT` 或 `REJECT` 导致流量未经代理，现自动切换到第一个策略组
- 修复订阅中未加引号的 REALITY `short-id`（如 `1234e123`）被误读为数字，导致订阅无法启用
- 修复订阅中唯一的代理组名为 `GLOBAL` 或没有代理组时，规则模式下代理页面显示「未获取到代理信息」

**🍎 macOS**

- 修复 macOS TUN 模式下原有 DNS 被清除、关闭后 DNS 未恢复及联网后设置未生效
- 修复 macOS 断网时误报代理已回退直连，恢复联网后自动重试系统代理

</details>

<details>
<summary><strong> ✨ 新增功能 </strong></summary>

- 新增连接页面按网络协议（TCP/UDP）筛选连接

</details>

<details>
<summary><strong> 🚀 优化改进 </strong></summary>

- 优化连接、规则和日志筛选：切换页面后保留过滤条件
- 优化全局模式代理列表：仅显示 `GLOBAL` 节点，选中其中的策略组时才显示该组

**🍎 macOS**

- 优化 macOS TUN 模式的系统 DNS，增加阿里和 Cloudflare 地址

</details>
