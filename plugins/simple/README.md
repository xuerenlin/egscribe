# Plugin Simple - 简单插件示例

这是一个使用 Rust 实现的 egscribe 插件示例，展示了如何创建一个基本的插件。

## 功能特性

该插件实现了以下功能：

- **echo**: 回显文本消息
- **greet**: 发送问候消息
- **get_info**: 获取插件信息

## 构建

在 `simple` 目录下运行：

```bash
cargo build --release
```

构建完成后，可执行文件位于 `target/release/simple`（Linux）或 `target/release/simple.exe`（Windows）。

## 配置

在 `plugins/simple/plugins.json` 中添加以下配置：

```json
{
  "id": "simple",
  "name": "简单插件示例",
  "exe_path": "./simple",
    "args": [],
    "config": {
      "greeting": "Hello"
    },
    "enabled": true,
    "auto_start": true
}
```

**注意**: 请根据实际构建路径调整 `exe_path`。

## 使用方法

插件启动后，主程序会发送初始化请求。插件会自动响应并发送就绪事件。

插件支持以下命令：

### echo 命令

回显文本消息：

```json
{
  "type": "request",
  "action": "execute",
  "id": "req_1",
  "command": "echo",
  "params": {
    "text": "Hello, World!"
  }
}
```

### greet 命令

发送问候消息：

```json
{
  "type": "request",
  "action": "execute",
  "id": "req_2",
  "command": "greet",
  "params": {
    "name": "egscribe"
  }
}
```

### get_info 命令

获取插件信息：

```json
{
  "type": "request",
  "action": "execute",
  "id": "req_3",
  "command": "get_info",
  "params": {}
}
```

## 消息协议

插件通过 stdin/stdout 与主程序通信，使用单行 JSON 格式的消息。

### 请求消息（主程序 -> 插件）

- `init`: 初始化插件
- `execute`: 执行命令
- `info`: 获取插件信息
- `shutdown`: 停止插件

### 响应消息（插件 -> 主程序）

插件会发送响应消息，包含请求ID、成功状态、数据或错误信息。

### 事件消息（插件 -> 主程序）

- `ready`: 插件就绪事件
- `notify`: 通知消息

## 开发说明

这是一个简单的插件示例，展示了：

1. 如何解析和处理来自主程序的 JSON 消息
2. 如何发送响应和事件消息
3. 如何实现基本的命令处理逻辑

你可以基于这个示例开发更复杂的插件功能。

## 许可证

MIT

