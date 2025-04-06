use clap::{Arg, ArgAction, Command};
use std::fs;
use std::process;
use kuchiki::traits::*;
use kuchiki::parse_html;

/// take html
fn extract_html_table(content: &str, header: &str) -> Option<String> {
    let start = content.find(header)?;
    let after_header = &content[start + header.len()..];
    let table_start = after_header.find("<table")?;
    let table_end = after_header[table_start..].find("</table>")?;
    Some(after_header[table_start..=table_start + table_end + 7].to_string())
}

/// parse html
fn parse_table(table_html: &str) -> Result<(Vec<String>, Vec<Vec<kuchiki::NodeRef>>, kuchiki::NodeRef), String> {
    let document = parse_html().one(table_html);
    let table_node = document
        .select_first("table")
        .map_err(|_| "Error selecting table".to_string())?
        .as_node()
        .clone();

    let mut headers = Vec::new();
    let mut rows = Vec::new();
    for (i, tr_match) in table_node
        .select("tr")
        .map_err(|_| "Error selecting tr".to_string())?
        .enumerate()
    {
        let tr_node = tr_match.as_node().clone();
        /// choose table elements
        let cell_nodes: Vec<kuchiki::NodeRef> = tr_node
            .select("th, td")
            .map_err(|_| "Error selecting th, td".to_string())?
            .map(|cell_match| cell_match.as_node().clone())
            .collect();

        if cell_nodes.is_empty() {
            continue;
        }

        if i == 0 {
            for cell in &cell_nodes {
                let text = cell.text_contents().trim().to_string();
                headers.push(text);
            }
        } else {
            rows.push(cell_nodes);
        }
    }

    if headers.is_empty() {
        return Err("No header row found in table".into());
    }
    Ok((headers, rows, table_node))
}


fn get_inner_html(node: &kuchiki::NodeRef) -> String {
    let mut html = Vec::new();
    for child in node.children() {
        child.serialize(&mut html).unwrap();
    }
    String::from_utf8(html).unwrap_or_default()
}


fn set_inner_html(node: &kuchiki::NodeRef, new_html: &str) {
    for child in node.children().collect::<Vec<_>>() {
        child.detach();
    }
    let fragment = parse_html().one(new_html);
    for child in fragment.children() {
        node.append(child);
    }
}

/// syncing
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("md-role-sync")
        .version("0.1.0")
        .author("you")
        .about("Синхронизирует содержимое HTML-таблиц между markdown файлами, не меняя их структуру")
        .arg(
            Arg::new("target")
                .long("target")
                .help("Путь к целевому markdown файлу")
                .required(true)
                .num_args(1),
        )
        .arg(
            Arg::new("source")
                .long("source")
                .help("Путь к исходному markdown файлу")
                .required(true)
                .num_args(1),
        )
        .arg(
            Arg::new("header")
                .long("header")
                .help("Markdown заголовок для поиска таблицы в обоих файлах")
                .conflicts_with_all(&["source-header", "target-header"])
                .num_args(1),
        )
        .arg(
            Arg::new("source-header")
                .long("header-source")
                .help("Markdown заголовок в исходном файле")
                .requires("target-header")
                .num_args(1),
        )
        .arg(
            Arg::new("target-header")
                .long("header-target")
                .help("Markdown заголовок в целевом файле")
                .requires("source-header")
                .num_args(1),
        )
        .arg(
            Arg::new("field")
                .long("field")
                .help("Соответствие в формате TargetField=SourceField")
                .required(true)
                .num_args(1..)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .help("Включить подробный вывод")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let target_path = matches.get_one::<String>("target").unwrap();
    let source_path = matches.get_one::<String>("source").unwrap();
    let header = matches.get_one::<String>("header");
    let source_header = matches.get_one::<String>("source-header");
    let target_header = matches.get_one::<String>("target-header");
    let verbose = matches.get_flag("verbose");

    // define header/s
    let (header_source, header_target) = match (header, source_header, target_header) {
        (Some(h), None, None) => (h.clone(), h.clone()),
        (None, Some(sh), Some(th)) => (sh.clone(), th.clone()),
        _ => {
            eprintln!("❌ Необходимо указать либо --header, либо и --header-source, и --header-target");
            process::exit(1);
        }
    };

    // parse elements
    let field_mappings: Vec<(String, String)> = matches
        .get_many::<String>("field")
        .unwrap()
        .map(|f| {
            let parts: Vec<&str> = f.split('=').collect();
            if parts.len() != 2 {
                eprintln!("❌ Неверный формат аргумента --field: {}", f);
                process::exit(1);
            }
            (parts[0].trim().to_string(), parts[1].trim().to_string())
        })
        .collect();

    if verbose {
        println!("📂 Target: {}", target_path);
        println!("📂 Source: {}", source_path);
        println!("🔎 Target header: {}", header_target);
        println!("🔎 Source header: {}", header_source);
        println!("🔁 Mappings: {:?}", field_mappings);
    }

    let target_content = fs::read_to_string(target_path)?;
    let source_content = fs::read_to_string(source_path)?;

    let target_table_html = extract_html_table(&target_content, &header_target)
        .ok_or("Таблица в целевом файле не найдена")?;
    let source_table_html = extract_html_table(&source_content, &header_source)
        .ok_or("Таблица в исходном файле не найдена")?;

    let (target_headers, mut target_rows, target_table_node) =
        parse_table(&target_table_html).map_err(|e| e.to_string())?;
    let (source_headers, source_rows, _) =
        parse_table(&source_table_html).map_err(|e| e.to_string())?;

    let target_role_index = target_headers
        .iter()
        .position(|h| h == "Роль")
        .ok_or("В целевой таблице нет столбца 'Роль'")?;
    let source_role_index = source_headers
        .iter()
        .position(|h| h == "Идентификатор роли")
        .ok_or("В исходной таблице нет столбца 'Идентификатор роли'")?;

    for target_row in target_rows.iter_mut() {
        if target_row.len() <= target_role_index {
            continue;
        }
        let role_value = target_row[target_role_index].text_contents().trim().to_string();

        if let Some(source_row) = source_rows.iter().find(|row| {
            if let Some(node) = row.get(source_role_index) {
                let node_text = node.text_contents().trim().to_string();
                node_text == role_value
            } else {
                false
            }
        }) {
            for (tgt_field, src_field) in &field_mappings {
                if let (Some(tgt_idx), Some(src_idx)) = (
                    target_headers.iter().position(|h| h == tgt_field),
                    source_headers.iter().position(|h| h == src_field),
                ) {
                    if let Some(source_cell) = source_row.get(src_idx) {
                        let new_content = get_inner_html(source_cell);
                        if let Some(target_cell) = target_row.get(tgt_idx) {
                            let current_content = get_inner_html(target_cell);
                            if current_content.trim() != new_content.trim() {
                                if verbose {
                                    println!(
                                        "🔄 Обновление '{}' для роли '{}':\n  '{}' → '{}'",
                                        tgt_field,
                                        role_value,
                                        current_content.trim(),
                                        new_content.trim()
                                    );
                                }
                                set_inner_html(target_cell, &new_content);
                            }
                        }
                    }
                }
            }
        }
    }

    let updated_table_html = target_table_node.to_string();

    if let Some(before_table_pos) = target_content.find(&header_target) {
        let after_header = &target_content[before_table_pos + header_target.len()..];
        if let Some(table_start) = after_header.find("<table") {
            if let Some(end_idx) = after_header[table_start..].find("</table>") {
                let table_end = table_start + end_idx + "</table>".len();
                let table_html_range = before_table_pos + header_target.len() + table_start
                    ..before_table_pos + header_target.len() + table_end;
                let mut new_content = target_content.clone();
                new_content.replace_range(table_html_range, &format!("\n\n{}", updated_table_html));
                fs::write(target_path, new_content)?;
                if verbose {
                    println!("✅ Обновлённая таблица записана в {}", target_path);
                }
            } else {
                eprintln!("❌ Не найден закрывающий тег </table> в целевом файле");
                process::exit(1);
            }
        } else {
            eprintln!("❌ Не найден тег <table> после заголовка в целевом файле");
            process::exit(1);
        }
    } else {
        eprintln!("❌ Не найден заголовок '{}' в целевом файле", header_target);
        process::exit(1);
    }

    Ok(())
}
