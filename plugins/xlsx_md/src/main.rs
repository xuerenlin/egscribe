use calamine::{open_workbook, Data, Reader, Xlsx};
use plugin_sdk::{run_plugin, PluginApi, PluginHandler, PluginMap};
use rust_xlsxwriter::Workbook;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

struct Xlsx2MdPlugin;

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        _ => cell.to_string(),
    }
}

fn md_escape(input: &str) -> String {
    input.replace('|', "\\|").replace('\n', "<br/>")
}

fn worksheet_to_markdown(sheet_name: &str, range: &calamine::Range<Data>) -> String {
    let (row_count, col_count) = range.get_size();
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", sheet_name));

    if row_count == 0 || col_count == 0 {
        out.push_str("_Empty sheet_\n\n");
        return out;
    }

    for row in 0..row_count {
        out.push('|');
        for col in 0..col_count {
            let text = range
                .get((row, col))
                .map(cell_to_string)
                .unwrap_or_default();
            out.push_str(&md_escape(&text));
            out.push('|');
        }
        out.push('\n');

        if row == 0 {
            out.push('|');
            for _ in 0..col_count {
                out.push_str("--|");
            }
            out.push('\n');
        }
    }

    out.push('\n');
    out
}

fn output_md_path(src: &Path) -> PathBuf {
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("xlsx_output");
    src.with_file_name(format!("{}.xlsx.md", stem))
}

fn convert_xlsx_to_md(path: &str) -> Result<PathBuf, String> {
    let src = Path::new(path);
    if !src.exists() {
        return Err(format!("File not found: {}", path));
    }

    let mut workbook: Xlsx<_> =
        open_workbook(src).map_err(|e| format!("Open workbook failed: {}", e))?;
    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err("Workbook has no worksheets".to_string());
    }

    let mut doc = String::new();
    doc.push_str(&format!("# XLSX Export\n\nSource: `{}`\n\n", path));

    let mut has_any_sheet = false;
    for name in sheet_names {
        match workbook.worksheet_range(&name) {
            Ok(range) => {
                has_any_sheet = true;
                doc.push_str(&worksheet_to_markdown(&name, &range));
            }
            Err(e) => {
                doc.push_str(&format!("# {}\n\n_Failed to read sheet: {}_\n\n", name, e));
            }
        }
    }

    if !has_any_sheet {
        return Err("No readable worksheet found".to_string());
    }

    let md_path = output_md_path(src);
    std::fs::write(&md_path, doc).map_err(|e| format!("Write markdown failed: {}", e))?;
    Ok(md_path)
}

fn output_xlsx_path(new_file_path: &str) -> Result<PathBuf, String> {
    let p = Path::new(new_file_path);
    if new_file_path.trim().is_empty() {
        return Err("Invalid new_file_path: empty".to_string());
    }
    let ext_is_xlsx = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("xlsx"))
        .unwrap_or(false);
    Ok(if ext_is_xlsx {
        p.to_path_buf()
    } else {
        p.with_extension("xlsx")
    })
}

fn split_md_row(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut escaped = false;
    let normalized = line.trim().trim_matches('|');

    for ch in normalized.chars() {
        if escaped {
            cur.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '|' {
            cells.push(cur.trim().replace("<br/>", "\n"));
            cur.clear();
            continue;
        }
        cur.push(ch);
    }
    cells.push(cur.trim().replace("<br/>", "\n"));
    cells
}

fn is_separator_row(cells: &[String]) -> bool {
    if cells.is_empty() {
        return false;
    }
    cells.iter().all(|cell| {
        let s = cell.trim();
        !s.is_empty() && s.chars().all(|c| c == '-' || c == ':' || c == ' ')
    })
}

fn parse_markdown_table(markdown: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            continue;
        }
        let cells = split_md_row(trimmed);
        if !cells.is_empty() && is_separator_row(&cells) {
            continue;
        }
        rows.push(cells);
    }

    if rows.is_empty() {
        return Err("current_outline_merged_table has no markdown table rows".to_string());
    }
    Ok(rows)
}

fn write_markdown_table_to_xlsx(
    markdown: &str,
    new_file_path: &str,
) -> Result<PathBuf, String> {
    let rows = parse_markdown_table(markdown)?;
    let output = output_xlsx_path(new_file_path)?;

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name("MergedTable")
        .map_err(|e| format!("Set worksheet name failed: {}", e))?;

    for (row_idx, row) in rows.iter().enumerate() {
        let r = u32::try_from(row_idx).map_err(|_| "Too many rows for xlsx".to_string())?;
        for (col_idx, cell) in row.iter().enumerate() {
            let c = u16::try_from(col_idx).map_err(|_| "Too many columns for xlsx".to_string())?;
            worksheet
                .write_string(r, c, cell)
                .map_err(|e| format!("Write xlsx cell failed: {}", e))?;
        }
    }

    workbook
        .save(&output)
        .map_err(|e| format!("Save xlsx failed: {}", e))?;
    Ok(output)
}

impl PluginHandler for Xlsx2MdPlugin {
    fn on_init(&mut self, api: &mut PluginApi, id: String, config: PluginMap) -> io::Result<()> {
        api.send_ready(
            "XLSX to Markdown",
            "0.1.0",
            vec![
                "hex_file_to_md".to_string(),
                "export_table_to_xlsx".to_string(),
            ],
        )?;
        api.send_ok(
            id,
            Some(serde_json::json!({
                "message": "xlsx_md plugin initialized",
                "config": config
            })),
        )
    }

    fn on_execute(
        &mut self,
        api: &mut PluginApi,
        id: String,
        command: String,
        params: PluginMap,
    ) -> io::Result<()> {
        match command.as_str() {
            "hex_file_to_md" => {
                let path = params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if path.is_empty() {
                    return api.send_err(id, "Missing param: path");
                }

                match convert_xlsx_to_md(&path) {
                    Ok(md_path) => {
                        let md_path_str = md_path.to_string_lossy().to_string();
                        let mut cmd_params = HashMap::new();
                        cmd_params.insert(
                            "path".to_string(),
                            serde_json::Value::String(md_path_str.clone()),
                        );
                        api.send_command("open_file", cmd_params)?;
                        api.send_ok(
                            id,
                            Some(serde_json::json!({
                                "source": path,
                                "output": md_path_str
                            })),
                        )
                    }
                    Err(e) => api.send_err(id, e),
                }
            }
            "export_table_to_xlsx" => {
                let markdown = params
                    .get("current_outline_merged_table")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if markdown.is_empty() {
                    return api.send_err(id, "Missing param: current_outline_merged_table");
                }

                let new_file_path = params
                    .get("new_file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if new_file_path.is_empty() {
                    return api.send_err(id, "Missing param: new_file_path");
                }

                match write_markdown_table_to_xlsx(&markdown, &new_file_path) {
                    Ok(output) => {
                        let output_str = output.to_string_lossy().to_string();
                        api.send_ok(
                            id,
                            Some(serde_json::json!({
                                "source": new_file_path,
                                "output": output_str
                            })),
                        )
                    }
                    Err(e) => api.send_err(id, e),
                }
            }
            _ => api.send_err(id, format!("Unknown command: {}", command)),
        }
    }
}

fn main() {
    let mut plugin = Xlsx2MdPlugin;
    run_plugin(&mut plugin);
}
