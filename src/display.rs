use crate::diff::{ChangeEntry, ChangeSection};
use owo_colors::OwoColorize;
use serde::Serialize;

/// Print all change sections with colored output.
pub fn print_changes(before_ref: &str, after_ref: &str, sections: &[ChangeSection]) {
    let total: usize = sections.iter().map(|s| s.entries.len()).sum();
    let section_count = sections.len();

    println!();
    println!(
        "  {} → {}  {}",
        before_ref.dimmed(),
        after_ref.bold(),
        format!("({total} changes across {section_count} sections)").dimmed(),
    );
    println!();

    for section in sections {
        print_section(section);
    }
}

fn print_section(section: &ChangeSection) {
    println!("  {}", section.name.bold().underline());
    println!();

    let mut added: Vec<(&str, Option<&str>)> = Vec::new();
    let mut removed: Vec<(&str, Option<&str>)> = Vec::new();
    let mut modified: Vec<(&str, &str)> = Vec::new();

    for entry in &section.entries {
        match entry {
            ChangeEntry::Added(n, d) => added.push((n, d.as_deref())),
            ChangeEntry::Removed(n, d) => removed.push((n, d.as_deref())),
            ChangeEntry::Modified(n, d) => modified.push((n, d)),
        }
    }

    for (name, desc) in &modified {
        println!(
            "    {} {}  {}",
            "~".yellow().bold(),
            name.yellow(),
            desc.dimmed()
        );
    }

    if !added.is_empty() && !removed.is_empty() {
        print_two_columns(&added, &removed);
    } else {
        for (name, detail) in &added {
            print_single_entry("+", name, *detail, true);
        }
        for (name, detail) in &removed {
            print_single_entry("-", name, *detail, false);
        }
    }

    println!();
}

/// Print added and removed entries side by side.
fn print_two_columns(added: &[(&str, Option<&str>)], removed: &[(&str, Option<&str>)]) {
    let max_left = added
        .iter()
        .map(|(n, d)| 6 + n.len() + d.map_or(0, |d| 2 + d.len()))
        .max()
        .unwrap_or(0);
    let right_col = (max_left + 4).clamp(28, 52);

    let rows = added.len().max(removed.len());
    for i in 0..rows {
        let used = if i < added.len() {
            print!("    {} {}", "+".green().bold(), added[i].0.green());
            let mut w = 6 + added[i].0.len();
            if let Some(d) = added[i].1 {
                print!("  {}", d.dimmed());
                w += 2 + d.len();
            }
            w
        } else {
            0
        };

        if i < removed.len() {
            let pad = right_col.saturating_sub(used).max(2);
            print!("{:w$}", "", w = pad);
            print!("{} {}", "-".red().bold(), removed[i].0.red());
            if let Some(d) = removed[i].1 {
                print!("  {}", d.dimmed());
            }
        }
        println!();
    }
}

fn print_single_entry(symbol: &str, name: &str, detail: Option<&str>, is_add: bool) {
    if is_add {
        print!("    {} {}", symbol.green().bold(), name.green());
    } else {
        print!("    {} {}", symbol.red().bold(), name.red());
    }
    if let Some(d) = detail {
        print!("  {}", d.dimmed());
    }
    println!();
}

// --- JSON export ---

#[derive(Serialize)]
struct JsonReport<'a> {
    before: &'a str,
    after: &'a str,
    total_changes: usize,
    sections: Vec<JsonSection<'a>>,
}

#[derive(Serialize)]
struct JsonSection<'a> {
    name: &'a str,
    changes: Vec<JsonChange>,
}

#[derive(Serialize)]
struct JsonChange {
    kind: &'static str,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

/// Serialize changes as JSON.
pub fn json_changes(before_label: &str, after_label: &str, sections: &[ChangeSection]) -> String {
    let report = JsonReport {
        before: before_label,
        after: after_label,
        total_changes: sections.iter().map(|s| s.entries.len()).sum(),
        sections: sections
            .iter()
            .map(|s| JsonSection {
                name: s.name,
                changes: s
                    .entries
                    .iter()
                    .map(|e| match e {
                        ChangeEntry::Added(name, detail) => JsonChange {
                            kind: "added",
                            name: name.clone(),
                            detail: detail.clone(),
                        },
                        ChangeEntry::Removed(name, detail) => JsonChange {
                            kind: "removed",
                            name: name.clone(),
                            detail: detail.clone(),
                        },
                        ChangeEntry::Modified(name, desc) => JsonChange {
                            kind: "modified",
                            name: name.clone(),
                            detail: Some(desc.clone()),
                        },
                    })
                    .collect(),
            })
            .collect(),
    };

    serde_json::to_string_pretty(&report).expect("failed to serialize JSON report")
}
