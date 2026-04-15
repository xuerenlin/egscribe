# 计算器插件 (cal)

这是一个计算器插件，可以自动计算以 `=` 开头的行中的数学表达式。

## 功能

- 注册 `LineChange` 触发器，监听行内容变化
- 当行内容以 `=` 开头时，自动计算 `=` 后面的表达式
- 将计算结果通过 `set_expanded_text` Action 同步给编辑器显示

## 使用方法

1. 在编辑器中输入以 `=` 开头的行，例如：
   ```
   = 1 + 1
   = 2 * 3
   = 10 / 2
   = sqrt(16)
   = sin(3.14159 / 2)
   ```

2. 插件会自动计算表达式并在该行下方显示结果

## 支持的表达式

插件使用 `evalexpr` 库进行计算，支持：
- 基本运算：`+`, `-`, `*`, `/`, `%`, `^` (幂运算)
- 数学函数：`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`
- 其他函数：`sqrt`, `exp`, `ln`, `log`, `abs`, `floor`, `ceil`, `round`
- 常量：`PI`, `E`
- 括号：支持使用括号改变运算优先级

## 配置

在 `plugins.json` 中配置插件：

```json
{
  "id": "cal",
  "name": "计算器插件",
  "description": "当行内容以 = 开头时，自动计算表达式并显示结果",
  "exe_path": "./cal",
  "args": [],
  "config": {},
  "enabled": true,
  "auto_start": true,
  "triggers": [
    "line_changed"
  ]
}
```

## 编译

```bash
cd plugins/cal
cargo build --release
```

编译后的可执行文件位于 `target/release/cal`。

