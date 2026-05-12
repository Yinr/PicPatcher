# PicPatcher

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Release](https://github.com/Yinr/PicPatcher/actions/workflows/release.yml/badge.svg)](https://github.com/Yinr/PicPatcher/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/Yinr/PicPatcher?label=release)](https://github.com/Yinr/PicPatcher/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Yinr/PicPatcher/total)](https://github.com/Yinr/PicPatcher/releases)
[![Platform](https://img.shields.io/badge/platform-Windows-blue)](https://github.com/Yinr/PicPatcher/releases)

[English](./README.md) | [中文](./README_zh.md)

批量图片覆盖工具：在目录下所有匹配的图片上，按固定像素坐标盖上一张固定的
覆盖图（例如日期水印）。

同一个 Rust workspace 产出两个可执行文件：

- `picpatcher`（CLI）— 由 JSON 配置文件驱动的批处理工具。
- `picpatcher-gui`（GUI）— 可视化编辑器（加载原图与覆盖图，拖拽定位，
  导出配置）并内置运行器，支持英文 / 简体中文界面。

## 构建

```
cargo build --release
```

产物位于 `target/release/`。
在 Windows 上，`picpatcher-gui.exe` 和 `picpatcher.exe` 都会将 `assets/icon.ico` 嵌入为文件图标。

## CLI 用法

```
picpatcher --config picpatcher.json --input ./photos --recursive
```

参数：

- `--config <FILE>` JSON 配置文件（可由 GUI 生成）。
- `--input <DIR>` 待扫描目录。
- `--ext jpg,png,…` 可选的扩展名过滤
  （默认：`jpg,jpeg,png,webp,bmp`）。
- `--recursive` 递归扫描子目录。
- `--in-place` 直接覆盖原文件（破坏性操作）。
- `--out-dir <NAME>` 非 in-place 模式下的同级输出目录名
  （默认：`_modified`）。
- `--dry-run` 只列出计划处理的输入/输出路径，不写入任何文件。

## 配置文件格式（v1）

```json
{
  "version": 1,
  "overlay": "./stamps/date.png",
  "x": 1200,
  "y": 80
}
```

`overlay` 可以是绝对路径，也可以是相对于配置文件所在目录的相对路径。

## GUI

运行 `picpatcher-gui`，包含两个标签页：

- **Editor（编辑器）**— 打开原图与覆盖图；缩放画布查看局部细节；
  在画布上拖拽覆盖图；使用方向键微调位置（`Shift` = 10 px）；微调 X/Y；
  保存 / 另存为配置文件。
- **Runner（运行器）**— 选择配置文件与输入目录；点击 Run；查看实时进度与错误日志；
  就地覆盖原文件时会弹出二次确认。
- **语言**— GUI 默认跟随系统语言，也可以在右上角通过 `EN / 简中` 手动切换。

## 说明

- JPEG 输出使用 95 质量。
- 带 Alpha 通道的覆盖图会按 Porter–Duff `over` 进行混合。
- PicPatcher 当前每份配置支持一个覆盖区域。多区域、缩放/旋转、EXIF 元数据保留
  暂不在 v1 范围内。

## 后续计划

未来版本的功能与想法记录在 [ROADMAP.md](./ROADMAP.md)。

## 许可证

MIT。详见 [LICENSE](./LICENSE)。
