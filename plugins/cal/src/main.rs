use plugin_sdk::{run_plugin, PluginApi, PluginHandler, PluginMap};
use std::collections::HashMap;
use std::io;

/// 预处理表达式，将不同进制的数字转换为十进制
/// 支持 0x (十六进制)、0o (八进制)、0b (二进制)
/// 同时支持删除 markdown 代码块格式的收尾行（```xxx```）
fn preprocess_expression(expr: &str) -> Result<String, String> {
    use regex::Regex;
    
    let mut result = expr.to_string();
    
    // 处理 markdown 代码块格式：删除首尾的 ``` 行
    let lines: Vec<&str> = result.lines().collect();
    if !lines.is_empty() {
        let first_line = lines[0].trim();
        let last_line = lines[lines.len() - 1].trim();
        
        // 检查是否是完整的代码块格式（首尾都是 ```）
        if first_line.starts_with("```") && last_line == "```" && lines.len() > 1 {
            // 删除首尾的 ``` 行
            result = lines[1..lines.len() - 1].join("\n");
        } else if first_line.starts_with("```") {
            // 如果只是以 ``` 开头，删除第一行
            result = lines[1..].join("\n");
        }
    }
    
    // 匹配十六进制数字 (0x 或 0X 开头)
    let hex_re = Regex::new(r"0[xX][0-9a-fA-F]+").map_err(|e| format!("正则表达式错误: {}", e))?;
    // 匹配八进制数字 (0o 或 0O 开头)
    let oct_re = Regex::new(r"0[oO][0-7]+").map_err(|e| format!("正则表达式错误: {}", e))?;
    // 匹配二进制数字 (0b 或 0B 开头)
    let bin_re = Regex::new(r"0[bB][01]+").map_err(|e| format!("正则表达式错误: {}", e))?;
    
    // 处理十六进制：使用 replace_all，对每个匹配进行转换
    result = hex_re.replace_all(&result, |caps: &regex::Captures| {
        let hex_str = caps.get(0).unwrap().as_str();
        match i64::from_str_radix(&hex_str[2..], 16) {
            Ok(num) => num.to_string(),
            Err(_) => hex_str.to_string(), // 如果解析失败，保持原样
        }
    }).to_string();
    
    // 处理八进制
    result = oct_re.replace_all(&result, |caps: &regex::Captures| {
        let oct_str = caps.get(0).unwrap().as_str();
        match i64::from_str_radix(&oct_str[2..], 8) {
            Ok(num) => num.to_string(),
            Err(_) => oct_str.to_string(),
        }
    }).to_string();
    
    // 处理二进制
    result = bin_re.replace_all(&result, |caps: &regex::Captures| {
        let bin_str = caps.get(0).unwrap().as_str();
        match i64::from_str_radix(&bin_str[2..], 2) {
            Ok(num) => num.to_string(),
            Err(_) => bin_str.to_string(),
        }
    }).to_string();
    
    Ok(result)
}

/// 存储不同进制的数值（不带前缀）
struct BaseValues {
    decimal: String,
    hexadecimal: String,
    octal: String,
    binary: String,
}

impl BaseValues {
    fn from_int(value: i64) -> Self {
        let dec = value.to_string();
        
        // 计算各种进制（不带前缀）
        let hex = if value >= 0 {
            format!("{:X}", value)
        } else {
            format!("{:X}", value as u64)
        };
        
        let oct = if value >= 0 {
            format!("{:o}", value)
        } else {
            format!("{:o}", value as u64)
        };
        
        let bin = if value >= 0 {
            format!("{:b}", value)
        } else {
            format!("{:b}", value as u64)
        };
        
        Self {
            decimal: dec,
            hexadecimal: hex,
            octal: oct,
            binary: bin,
        }
    }
    
    /// 从浮点数创建 BaseValues
    /// 第一列（decimal）显示完整的浮点数，其他列显示整数部分的进制数
    fn from_float(value: f64) -> Self {
        let dec = value.to_string();
        let int_part = value.trunc() as i64;
        
        // 计算各种进制（基于整数部分，不带前缀）
        let hex = if int_part >= 0 {
            format!("{:X}", int_part)
        } else {
            format!("{:X}", int_part as u64)
        };
        
        let oct = if int_part >= 0 {
            format!("{:o}", int_part)
        } else {
            format!("{:o}", int_part as u64)
        };
        
        let bin = if int_part >= 0 {
            format!("{:b}", int_part)
        } else {
            format!("{:b}", int_part as u64)
        };
        
        Self {
            decimal: dec,
            hexadecimal: hex,
            octal: oct,
            binary: bin,
        }
    }
}

/// 格式化 Value 为字符串（简单格式，用于非数字类型的后备显示）
fn format_value_with_bases(value: &evalexpr::Value) -> String {
    match value {
        evalexpr::Value::Int(i) => i.to_string(),
        evalexpr::Value::Float(f) => f.to_string(),
        evalexpr::Value::Boolean(b) => b.to_string(),
        evalexpr::Value::String(s) => s.clone(),
        evalexpr::Value::Tuple(t) => {
            let items: Vec<String> = t.iter().map(format_value_with_bases).collect();
            format!("({})", items.join(", "))
        }
        evalexpr::Value::Empty => "".to_string(),
    }
}


/// 从表达式中提取变量名（通过查找赋值语句）
fn extract_variable_names(expr: &str) -> Vec<String> {
    use regex::Regex;
    
    // 匹配赋值语句，如 "a = 5", "b = a + 1" 等
    // 匹配模式：标识符 = 表达式
    let re = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\s*=").ok();
    
    if let Some(re) = re {
        let mut vars = Vec::new();
        for cap in re.captures_iter(expr) {
            if let Some(var_name) = cap.get(1) {
                let var = var_name.as_str().to_string();
                if !vars.contains(&var) {
                    vars.push(var);
                }
            }
        }
        vars
    } else {
        Vec::new()
    }
}

/// 从 Value 中提取各进制的值（支持整数和浮点数）
fn extract_base_values(value: &evalexpr::Value) -> Option<BaseValues> {
    match value {
        evalexpr::Value::Int(i) => Some(BaseValues::from_int(*i)),
        evalexpr::Value::Float(f) => {
            if f.fract() == 0.0 {
                // 如果是整数，使用整数格式化
                Some(BaseValues::from_int(*f as i64))
            } else {
                // 如果是浮点数，使用浮点数格式化（第一列显示浮点数，其他列显示整数部分）
                Some(BaseValues::from_float(*f))
            }
        }
        _ => None,
    }
}

/// 生成 Markdown 表格
fn generate_markdown_table(rows: Vec<(String, BaseValues)>) -> String {
    if rows.is_empty() {
        return String::new();
    }
    
    let mut table_lines = Vec::new();
    
    // 表头
    table_lines.push("|Variable|Decimal|Hex|Oct|Bin|".to_string());
    table_lines.push("|--|--|--|--|--|".to_string());
    
    // 表格行
    for (name, bases) in rows {
        table_lines.push(format!(
            "|{}|{}|{}|{}|{}|",
            name, bases.decimal, bases.hexadecimal, bases.octal, bases.binary
        ));
    }
    
    table_lines.join("\n")
}

/// 计算表达式，返回 Markdown 表格形式的结果
/// 支持多行表达式（用分号或换行符分隔）和变量
fn evaluate_expression(expr: &str) -> Result<String, String> {
    use evalexpr::*;
    
    // 预处理不同进制的数字
    let mut processed_expr = preprocess_expression(expr)?;
    
    // 将换行符转换为分号（evalexpr 使用分号分隔多个表达式）
    // 但要小心处理，避免将字符串中的换行符也转换了
    // 这里简单处理：将独立的换行符（前后有空格或行首/行尾）替换为分号
    processed_expr = processed_expr
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("; ");
    
    // 提取表达式中的变量名（用于后续显示）
    let var_names = extract_variable_names(&processed_expr);
    
    // 创建上下文用于存储变量
    let mut context = HashMapContext::new();
    
    // 评估表达式（支持多行，用分号分隔）
    // evalexpr 使用分号分隔多个表达式，最后一个表达式的值作为返回值
    let result = eval_with_context_mut(&processed_expr, &mut context)
        .map_err(|e| format!("计算错误: {}", e))?;
    
    // 构建表格行数据
    let mut table_rows = Vec::new();
    
    // 添加结果行
    if let Some(bases) = extract_base_values(&result) {
        table_rows.push(("Result".to_string(), bases));
    } else {
        // 如果结果不是数字类型，使用简单格式
        return Ok(format_value_with_bases(&result));
    }
    
    // 添加变量行
    for var_name in var_names {
        if let Some(value) = context.get_value(&var_name) {
            if let Some(bases) = extract_base_values(value) {
                table_rows.push((var_name, bases));
            }
        }
    }
    
    // 生成 Markdown 表格
    Ok(generate_markdown_table(table_rows))
}

struct CalculatorPlugin;

impl PluginHandler for CalculatorPlugin {
    fn on_init(&mut self, api: &mut PluginApi, id: String, config: PluginMap) -> io::Result<()> {
        api.send_ready("Calculator Plugin", "0.1.0", vec!["calculate".to_string()])?;
        let response_data = serde_json::json!({
            "message": "Calculator plugin initialized successfully",
            "config": config
        });
        api.send_ok(id, Some(response_data))
    }

    fn on_execute(
        &mut self,
        api: &mut PluginApi,
        id: String,
        command: String,
        _params: PluginMap,
    ) -> io::Result<()> {
        api.send_err(id, format!("Unknown command: {}", command))
    }

    fn on_shutdown(&mut self, api: &mut PluginApi, id: String) -> io::Result<()> {
        api.notify("info", "Calculator plugin is shutting down...")?;
        api.send_ok(id, None)
    }

    fn on_notify(
        &mut self,
        api: &mut PluginApi,
        id: String,
        event_type: String,
        data: PluginMap,
    ) -> io::Result<()> {
        let mut error_msg: Option<String> = None;

        if event_type.as_str() == "line_changed" {
            let line_no = data.get("line_no").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let line_text = data
                .get("line_text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let trimmed = line_text.trim_start();
            let expr = if trimmed.starts_with("```") {
                line_text.trim()
            } else if trimmed.starts_with('=') {
                trimmed[1..].trim()
            } else {
                ""
            };

            if !expr.is_empty() {
                match evaluate_expression(expr) {
                    Ok(result_str) => {
                        let mut params = HashMap::new();
                        params.insert(
                            "line_no".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(line_no)),
                        );
                        params.insert(
                            "expanded_text".to_string(),
                            serde_json::Value::String(result_str),
                        );

                        if let Err(e) = api.send_command("set_expanded_text", params) {
                            error_msg = Some(format!("发送计算结果失败: {}", e));
                        }
                    }
                    Err(e) => {
                        error_msg = Some(format!("错误: {}", e));
                    }
                }
            }
        }

        if let Some(err) = error_msg {
            api.send_err(id, err)
        } else {
            api.send_ok(
                id,
                Some(serde_json::json!({
                    "event_type": event_type,
                    "handled": true
                })),
            )
        }
    }
}

fn main() {
    let mut plugin = CalculatorPlugin;
    run_plugin(&mut plugin);
}

