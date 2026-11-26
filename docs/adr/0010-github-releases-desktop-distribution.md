# 通过 GitHub Releases 分发桌面安装包

CodeLab Studio 使用 GitHub Releases 作为桌面安装包的唯一分发源。以后新增的下载入口可直接引用同一份 Release 附件，避免维护两套版本和安装包。版本标签触发所有目标平台的构建，流水线收齐安装包后创建 Release 草稿，由维护者验收并发布。首发采用 Public Preview，让早期用户可以下载试用，并在发布说明中明确 macOS 公证和 Windows 签名状态。
