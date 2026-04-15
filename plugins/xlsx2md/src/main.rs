use calamine::{open_workbook, Data, Reader, Xlsx};
use plugin_sdk::{run_plugin, PluginApi, PluginHandler, PluginMap};
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
    out.push_str(&format!("## {}\n\n", sheet_name));

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
                doc.push_str(&format!("## {}\n\n_Failed to read sheet: {}_\n\n", name, e));
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

impl PluginHandler for Xlsx2MdPlugin {
    fn on_init(&mut self, api: &mut PluginApi, id: String, config: PluginMap) -> io::Result<()> {
        api.send_ready("XLSX to Markdown", "0.1.0", vec!["hex_file_to_md".to_string()])?;
        api.send_ok(
            id,
            Some(serde_json::json!({
                "message": "xlsx2md plugin initialized",
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
            _ => api.send_err(id, format!("Unknown command: {}", command)),
        }
    }
}

fn main() {
    let mut plugin = Xlsx2MdPlugin;
    run_plugin(&mut plugin);
}
