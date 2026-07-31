//! `complai parse` 的 xlsx 解析测试:用 rust_xlsxwriter 生成夹具,再解析核对。

use complai::parse::parse_xlsx;

#[test]
fn parse_xlsx_returns_markdown_tables() {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("资产").unwrap();
    ws.write_string(0, 0, "资产名称").unwrap();
    ws.write_string(0, 1, "类型").unwrap();
    ws.write_string(1, 0, "用户数据库").unwrap();
    ws.write_string(1, 1, "MySQL").unwrap();
    ws.write_number(2, 0, 42.0).unwrap();

    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("sample.xlsx");
    wb.save(&path).unwrap();

    let md = parse_xlsx(&path).unwrap();

    assert!(md.contains("## 资产"));
    assert!(md.contains("资产名称"));
    assert!(md.contains("用户数据库"));
    assert!(md.contains("MySQL"));
    // 数值单元格被转为字符串。
    assert!(md.contains("42"));
    // 表头分隔行。
    assert!(md.contains("---"));
}

#[test]
fn parse_xlsx_escapes_pipe_in_cells() {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_string(0, 0, "列").unwrap();
    ws.write_string(1, 0, "a|b").unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("pipe.xlsx");
    wb.save(&path).unwrap();

    let md = parse_xlsx(&path).unwrap();
    assert!(md.contains("a\\|b"));
    assert!(!md.contains("a|b\n"));
}
