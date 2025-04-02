use anyhow::{anyhow, Result};
use clap::Parser;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// CLI args
#[derive(Parser, Debug)]
#[command(name = "md-role-sync", about = "Синхронизация описаний ролей из другого Markdown-файла")]
struct Args {
    /// path to target
    #[arg(long)]
    target: PathBuf,

    /// path to source
    #[arg(long)]
    source: PathBuf,

    /// logs
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let target_md = fs::read_to_string(&args.target)?;
    let source_md = fs::read_to_string(&args.source)?;

    let updated_md = sync_roles(
        &target_md,
        &source_md,
        "### Базовые проектные роли {#список-проектных-ролей}",
        "### Базовые проектные роли",
        args.verbose,
    )?;

    fs::write(&args.target, updated_md)?;
    println!("Таблица успешно обновлена в {:?}", args.target);
    Ok(())
}

fn sync_roles(
    target_md: &str,
    source_md: &str,
    target_heading: &str,
    source_heading: &str,
    verbose: bool,
) -> Result<String> {
    let source_table_html = extract_table_after_heading(source_md, source_heading)?;
    let source_roles = extract_role_map(&source_table_html)?;

    if verbose {
        println!("Извлечено ролей из source: {}", source_roles.len());
    }

    let target_table_html = extract_table_after_heading(target_md, target_heading)?;
    let fragment = Html::parse_fragment(&target_table_html);
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();

    let mut updated_count = 0;
    let mut new_rows = vec![];

    for row in fragment.select(&row_selector) {
        let cells: Vec<_> = row.select(&cell_selector).collect();
        let mut row_html = String::new();
        row_html.push_str("<tr>");

        if cells.len() >= 3 {
            let role_id = cells[0].text().collect::<String>().trim().to_string();
            if let Some(new_desc) = source_roles.get(&role_id) {
                let old_desc = cells[1].text().collect::<String>().trim().to_string();
                let note = cells[2].text().collect::<String>().trim().to_string();

                if old_desc != *new_desc {
                    updated_count += 1;
                    if verbose {
                        println!("Обновляем '{}':\n  Было: {}\n  Стало: {}\n", role_id, old_desc, new_desc);
                    }
                }

                row_html.push_str(&format!("<td>{}</td><td>{}</td><td>{}</td>", role_id, new_desc, note));
                row_html.push_str("</tr>");
                new_rows.push(row_html);
                continue;
            }
        }

        for cell in cells {
            let content = cell.text().collect::<String>().trim().to_string();
            row_html.push_str(&format!("<td>{}</td>", content));
        }

        row_html.push_str("</tr>");
        new_rows.push(row_html);
    }

    if verbose {
        println!("Обновлено строк: {}", updated_count);
    }

    let new_table_html = format!("<table>\n{}\n</table>", new_rows.join("\n"));

    let heading_pos = target_md.find(target_heading)
        .ok_or_else(|| anyhow!("Не найден заголовок в target: {}", target_heading))?;

    let after_heading = &target_md[heading_pos..];
    let table_start = after_heading.find("<table>")
        .ok_or_else(|| anyhow!("Не найдена <table> после заголовка"))?;
    let table_end = after_heading.find("</table>")
        .ok_or_else(|| anyhow!("Не найдена </table> после заголовка"))? + "</table>".len();

    let before_table = &target_md[..heading_pos + table_start];
    let after_table = &after_heading[table_end..];

    let final_md = format!("{}{}\n{}", before_table, new_table_html, after_table);
    Ok(final_md)
}

fn extract_table_after_heading(md: &str, heading: &str) -> Result<String> {
    let heading_pos = md.find(heading)
        .ok_or_else(|| anyhow!("Не найден заголовок: '{}'", heading))?;

    let html_after = &md[heading_pos..];
    let document = Html::parse_fragment(html_after);
    let table_selector = Selector::parse("table").unwrap();

    let table = document
        .select(&table_selector)
        .next()
        .ok_or_else(|| anyhow!("Не найдена таблица после заголовка"))?;

    Ok(table.html())
}

fn extract_role_map(table_html: &str) -> Result<HashMap<String, String>> {
    let fragment = Html::parse_fragment(table_html);
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();

    let mut map = HashMap::new();

    for row in fragment.select(&row_selector).skip(1) {
        let cells: Vec<_> = row.select(&cell_selector).collect();
        if cells.len() >= 3 {
            let id = cells[0].text().collect::<String>().trim().to_string();
            let desc = cells[2].text().collect::<String>().trim().to_string();
            map.insert(id, desc);
        }
    }

    Ok(map)
}
