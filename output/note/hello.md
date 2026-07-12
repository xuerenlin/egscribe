# egscribe Help

# Test2
## Test Here
[text](url)Test
egscribe is a local-first Markdown note-taking app and lightweight text editor, built around the core principle of **managing notes within the `note` directory**, while also supporting the opening of external plain text files.
2026-05-04 00:02:16

## Test AA
```plantuml file://E:/rustspace/egscribe/target/debug/cache/plantuml/code_1_14900028107393667268.png
@startuml
skinparam participantStyle rectangle
participant 生产日期字段适配
database 接口消息表
database 清洗规则配置表
database 产品信息表
database 接口响应表

生产日期字段适配 -> 接口消息表 : 读取生产日期上报消息
接口消息表 --> 生产日期字段适配 : 返回生产日期、消息ID、时间戳
生产日期字段适配 -> 清洗规则配置表 : 读取清洗规则
清洗规则配置表 --> 生产日期字段适配 : 返回规则ID、字段名称、格式要求
生产日期字段适配 -> 产品信息表 : 写入清洗后生产日期
生产日期字段适配 -> 接口响应表 : 写入处理结果
@enduml
```

---
## Quick Start

1. After launch, the app automatically creates:
   - `note/`: notes directory
   - `note/images/`: pasted image storage directory
   - `note/config.json`: configuration file
2. Create a new note in the left **Notes** panel. It is saved as `note/<name>.md` by default.
3. Press `Ctrl+S` to save. If auto-save is enabled, data is saved at the configured interval.
>For first-time use, open this file and follow along while trying each feature.


```plantuml file://E:/rustspace/egscribe/target/debug/cache/plantuml/code_2_14680482145709108849.png
@startuml
participant "模块AC" as A
participant "模块B" as B
participant "模块" as C

A -> B: 数据属性传递
@enduml
```


---

## Interface Overview

The Markdown Editor interface is divided into four main areas, designed for efficient document editing and management.

### Top Toolbar

- UI language switch (Chinese / English)
- Theme switch (Light / Dark)
- Text brightness
- Font size
- Line number toggle
- Table style (Full border / Horizontal only / No border)
- Table row index column toggle
- Markdown heading section number toggle
- Wrap text toggle
- Indent decrease/increase
- New file

### Left Side Panel

There are three tabs on the left:

- **Notes**: create, rename, delete, refresh, and pin frequently used notes
- **Outline**: show headings of the current Markdown document and click to jump
- **Plugins**: view plugin status, start/stop plugins, and view logs

Use `Esc` to quickly show/hide the side panel.

### Center Editor Area

- Supports multiple tabs (notes and external files are managed separately)
- Supports drag-and-drop file opening
- Top path bar helps switch between related notes/files

### Bottom Status Bar


- Shows whether the current tab is a **Note** or **File**
- Syntax highlight language switch (including Plain Text)
- Selection info display
- Line ending settings (CRLF/LF, etc.) and save
- Reopen or save with a specified encoding
## Common Shortcuts
|Shortcut|Action|
|--|--|
|`Ctrl+S`|Save current document|
|`Ctrl+F`|Open find/replace window (tries to preload selected text)|
|`Ctrl+Z` / `Ctrl+Y`|Undo / Redo|
|`Ctrl+A`|Select al|
|More|Check corresponding shortcuts in the context menu|

**The context menu supports common Markdown insertions**: headings, bold, italic, strikethrough, links, inline code, lists, TODO, quote, table, code block, horizontal rule, etc.

---

## Notes and Wiki Links

### Difference Between Notes and Files

- **Note**: stored under `note/*.md`, managed by note name
- **File**: external plain text file opened by path

### Test


### Wiki Link Syntax

Use `[[WikiLinkedNote]]` to create links between notes (without `.md` suffix). Click the link icon to auto-create and jump to the target note.  
Wiki links are indexed in note relationships, making navigation easier in your knowledge base.

### Pin Frequently Used Notes

You can pin commonly used notes for quick access in the tab area.

|Shortcut||Action|
|--|--|--|
|`Ctrl+S`|sdf|Save current document|
|`Ctrl+F`||Open find/replace window (tries to preload selected text)|
|`Ctrl+Z` / `Ctrl+Y`||Undo / Redo|
|`Ctrl+A`||Select all|
|`Ctrl+Enter`||Exit table/code block and continue normal paragraph|
|`Esc`||Show/Hide left side panel|
|More||Check corresponding shortcuts in the context menu|

---

## Find and Replace

The find window supports:
- Find in current document
- Find all in current document
- Replace / Replace all
- Search across notes
- Case-sensitive / Whole-word / Regex

You can click results in the result panel to jump directly.


---

## Markdown Editing Features

You can copy the examples below directly into the editor.

### Text Formatting
This is plain text.  
This is **bold** text.  
This is *italic* text.  
This is ~~strikethrough~~ text.  
This is `inline code`.

### Lists (Unordered / Ordered / TODO)
- Unordered item A
- Unordered item B
    - Sub item B.1
    - Sub item B.2

1. Ordered item 1
2. Ordered item 2

- [ ] TODO item
- [x] Done item
- [x] Done item

### Quote, Horizontal Rule, Link, Image

>This is a quote block
> Second quote line


---
[egscribe Project on Gitee](https://gitee.com/linxueren_0/egscribe)

[egscribe Project on GitHub](https://github.com/xuerenlin/egscribe)


![Sample Image](image_019d9074-2c69-7282-9a0b-2f9faa6c3edc.png)

Notes:

- External links can be opened with the link icon in your system browser.
- Pasting an image (`Ctrl+V`) writes it into `note/images/` and inserts image syntax automatically.

### Wiki Link Notes

Example:
- Today I organized Rust study notes and linked [[Daily Notes]] and [[Development Plan]].

Notes:
- Wiki link syntax is `[[NoteName]]` without `.md`.
- Useful for building a navigable knowledge graph.

### 按时

#### Ctrl+S
Description
Save current docu
### Tables

Basic table example:
FeatureShortcutDescription
SaveCtrl+SSave current document
FindCtrl+FOpen find/replace
Side panelEscShow/Hide side panel

Table editing notes:
- Once recognized as a table, visual editing is available (insert/delete rows and columns).
- Toolbar supports border style switching (full / horizontal only / none).
- Table row index column can be toggled.
- Pressing Enter usually continues editing inside the table. Use `Ctrl+Enter` to continue normal paragraph text.

### Code Blocks

Plain fenced code block:
This is a plain text code block
Second line

Rust example (syntax highlight):
fn main() {
    println!("Hello, egscribe!");
}

Python example:
def add(a, b):
    return a + b

Notes:
- Fenced code block format: triple backticks + language + content + triple backticks.
- Pressing Enter continues inside the code block. Use `Ctrl+Enter` to return to normal paragraph text.

## Auto Save and Configuration

Main config is in `note/config.json`, including:

- Theme, font size, wrapping, line numbers, and other display options
- Recent files, pinned notes, tab scrollbar display
- Default encoding and auto encoding detection
- Auto-save enable switch and interval (seconds)
- Table style and heading section number display

If you prefer manual control, disable auto-save. If you write frequently, auto-save is recommended.



---

## Plugin System

Plugin directory is `plugins/` next to the executable.  
Each plugin subdirectory should contain `desc.json` and an executable.

Built-in visible sample plugins:
- `cal`: auto-calculate when line starts with `=` or in `Math` fenced code blocks
- `xlsx2md`: converts xlsx/xlsm files to Markdown and opens them
- `simple` / `simple_py`: sample plugins

In the **Plugins** tab (left panel), you can view status/logs and manually start/stop plugins.



---

## Single Instance and File Opening Behavior

- The app runs in single-instance mode
- Launching again forwards the target file path to the running instance
- Supports command-line file path input
- Supports drag-and-drop file opening
- Some non-text files may prompt plugin-based handling

---

## Advanced: Password Links (Local Encrypted Text)

Supported password link format:

`[Title](passwd:your_password "plain_text_content")`

The app can encrypt content as `cipher:` prefixed data and decrypt it when the correct password is provided.  
This is suitable for lightweight local privacy, but it is not a replacement for a professional password manager.
2026-05-04 00:02:36
2026-05-04 00:02:57
