//! `complai parse <file>`:把外部文档抽成纯文本/表格,供 agent 灌库。
//!
//! 目前支持 `.xlsx`(用 calamine 读所有工作表,输出为 Markdown 表格,首行作表头)。
//! 这一步是确定性的"字节 -> 文本/结构";"理解内容并映射成 facts/控制项"是 agent 的活
//! (见 `doc-ingest` skill)。

use std::path::Path;

use calamine::{open_workbook, Data, Reader, Xlsx};
use eyre::WrapErr;

pub fn parse(file: &str) -> eyre::Result<()> {
    let path = Path::new(file);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let out = match ext {
        "xlsx" => parse_xlsx(path)?,
        other => eyre::bail!("不支持的格式 `.{other}`:目前仅支持 .xlsx"),
    };
    print!("{out}");
    Ok(())
}

/// 读 xlsx 所有工作表,返回 Markdown 表格字符串(每表一个 `## <表名>` 段)。
pub fn parse_xlsx(path: &Path) -> eyre::Result<String> {
    let mut workbook: Xlsx<_> = open_workbook(path)
        .wrap_err_with(|| format!("打开 xlsx {} 失败", path.display()))?;
    let sheets = workbook.sheet_names().clone();

    let mut out = String::new();
    for sheet in &sheets {
        out.push_str(&format!("## {sheet}\n\n"));
        let range = workbook
            .worksheet_range(sheet)
            .wrap_err_with(|| format!("读取工作表 {sheet} 失败"))?;
        let mut rows = range.rows();
        // 首行作表头;空表则跳过。
        if let Some(header) = rows.next() {
            let cols: Vec<String> = header.iter().map(cell_to_string).collect();
            out.push_str(&format!("| {} |\n", cols.join(" | ")));
            out.push_str(&format!(
                "| {} |\n",
                cols.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")
            ));
        }
        for row in rows {
            let cells: Vec<String> = row.iter().map(cell_to_string).collect();
            out.push_str(&format!("| {} |\n", cells.join(" | ")));
        }
        out.push('\n');
    }
    Ok(out)
}

/// 单元格转字符串;转义 `|` 与换行以免破坏 Markdown 表格。
fn cell_to_string(d: &Data) -> String {
    let s = match d {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => f.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(d) => format!("{d}"),
        Data::Error(e) => format!("{e:?}"),
        Data::DurationIso(s) => s.clone(),
        Data::DateTimeIso(s) => s.clone(),
    };
    s.replace('|', "\\|").replace(['\n', '\r'], " ")
}
