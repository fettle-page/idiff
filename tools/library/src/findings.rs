//! Counts for `receipts/FINDINGS.md`, from a downloaded catalog run
//! (`result-*/<id>/{meta,receipt,leftovers}.json`, or `<id>/…` directly).
//!
//! Every count comes from the receipts' own lines, recognised by the same `copy` templates that
//! wrote them. Apps are listed alphabetically, each with a link to its page: no rankings, no
//! scores, no words about vendors.

use crate::{Entry, Meta, id_slug, safe_run_url, uninstall_unfinished, version_slug};
use idiff_core::copy;
use idiff_core::render::md_escape;
use idiff_core::verdict::{Level, Receipt, format_size};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// One app of the run, with what its lines say.
struct App {
    name: String,
    id: String,
    link: String,
    services: bool,
    daily_tasks: bool,
    tasks: bool,
    startup: bool,
    inbound: bool,
    drivers: bool,
    certs: bool,
    /// False when the uninstall check didn't finish, so the leftovers mean nothing.
    checked: bool,
    left: bool,
    /// Total of the leftover folders that have a size (as the lines round them), if any.
    left_bytes: Option<u64>,
}

/// Whether `text` fits `template`: its literal parts in order, each `{placeholder}` standing for
/// anything.
fn fits(text: &str, template: &str) -> bool {
    let mut parts = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        parts.push(&rest[..open]);
        match rest[open..].find('}') {
            Some(close) => rest = &rest[open + close + 1..],
            None => break,
        }
    }
    parts.push(rest);
    let Some(mut rest) = text.strip_prefix(parts[0]) else {
        return false;
    };
    if parts.len() == 1 {
        return rest.is_empty();
    }
    for mid in &parts[1..parts.len() - 1] {
        match rest.find(mid) {
            Some(i) => rest = &rest[i + mid.len()..],
            None => return false,
        }
    }
    rest.ends_with(parts[parts.len() - 1])
}
fn any_line(r: &Receipt, level: Level, templates: &[&str]) -> bool {
    r.lines
        .iter()
        .any(|l| l.level == level && templates.iter().any(|t| fits(&l.text, t)))
}

/// A tasks line whose summary says one of the tasks runs every day.
fn daily_task_line(r: &Receipt) -> bool {
    r.lines.iter().any(|l| {
        l.level == Level::Warn
            && [copy::TASKS_ONE, copy::TASKS_MANY].iter().any(|t| fits(&l.text, t))
            && l.text.split_once(": ").is_some_and(|(_, summary)| {
                summary
                    .split("; ")
                    .any(|s| s.contains(&format!(" {}", copy::WHEN_DAILY)))
            })
    })
}

/// "512 KB in AppData\…" is `FOLDER_LEFT`: the bytes the line rounds to.
fn folder_bytes(text: &str) -> Option<u64> {
    if !fits(text, copy::FOLDER_LEFT) {
        return None;
    }
    let (size, _) = text.split_once(" in ")?;
    let (num, unit) = size.split_once(' ')?;
    let unit = match unit {
        "KB" => 1024.0,
        "MB" => 1024.0 * 1024.0,
        "GB" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((num.parse::<f64>().ok()? * unit).round() as u64)
}

fn leftovers_unfinished(left: Option<&Receipt>) -> bool {
    left.is_none_or(|l| {
        any_line(
            l,
            Level::Warn,
            &[copy::UNINSTALL_TIMED_OUT, copy::UNINSTALL_DIDNT_FINISH],
        )
    })
}

/// Every app dir under `root`: `root/<id>/meta.json` or `root/<artifact>/<id>/meta.json`.
fn app_dirs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let subdirs = |dir: &Path| -> Result<Vec<PathBuf>, String> {
        let mut found: Vec<PathBuf> = fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        found.sort();
        Ok(found)
    };
    let mut dirs = Vec::new();
    for first in subdirs(root)? {
        if first.join("meta.json").is_file() {
            dirs.push(first);
        } else {
            dirs.extend(subdirs(&first)?.into_iter().filter(|p| p.join("meta.json").is_file()));
        }
    }
    Ok(dirs)
}

fn read<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Result<Option<T>, String> {
    crate::read_json(path)
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The Markdown for `receipts/FINDINGS.md`. With `receipts_dir` (holding `index.json`), links
/// are the pages the library really wrote; without it they're the paths it would write.
pub fn findings_md(results_dir: &Path, receipts_dir: Option<&Path>) -> Result<String, String> {
    let index: Vec<Entry> = match receipts_dir {
        Some(dir) => read(&dir.join("index.json"))?.unwrap_or_default(),
        None => Vec::new(),
    };
    let mut apps = Vec::new();
    let mut metas: Vec<Meta> = Vec::new();
    let mut not_installed: Vec<(String, String)> = Vec::new();
    for dir in app_dirs(results_dir)? {
        let dir_name = dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let Some(meta) = read::<Meta>(&dir.join("meta.json"))? else {
            continue;
        };
        let id = if meta.id.is_empty() { dir_name } else { meta.id.clone() };
        let name = if meta.name.is_empty() {
            id.clone()
        } else {
            meta.name.clone()
        };
        let unfinished = uninstall_unfinished(&meta);
        let receipt: Option<Receipt> = read(&dir.join("receipt.json"))?;
        let Some(receipt) = receipt.filter(|_| meta.failed.is_none() || unfinished) else {
            not_installed.push((name, id));
            continue;
        };
        let left_receipt: Option<Receipt> = read(&dir.join("leftovers.json"))?;
        let checked = !unfinished && !leftovers_unfinished(left_receipt.as_ref());
        let left_lines: Vec<&str> = left_receipt
            .iter()
            .filter(|_| checked)
            .flat_map(|l| l.lines.iter())
            .filter(|l| l.level == Level::Bad)
            .map(|l| l.text.as_str())
            .collect();
        let sizes: Vec<u64> = left_lines.iter().filter_map(|t| folder_bytes(t)).collect();
        let link = index
            .iter()
            .find(|e| e.id == id && e.version == meta.version)
            .map(|e| crate::safe_path(&e.path))
            .unwrap_or_else(|| format!("{}/{}.md", id_slug(&id), version_slug(&meta.version)));
        apps.push(App {
            link,
            services: any_line(&receipt, Level::Warn, &[copy::SERVICES_ONE, copy::SERVICES_MANY]),
            daily_tasks: daily_task_line(&receipt),
            tasks: any_line(&receipt, Level::Warn, &[copy::TASKS_ONE, copy::TASKS_MANY]),
            startup: any_line(&receipt, Level::Warn, &[copy::STARTS_WITH_WINDOWS]),
            inbound: any_line(&receipt, Level::Warn, &[copy::FIREWALL_INBOUND]),
            drivers: any_line(&receipt, Level::Warn, &[copy::DRIVERS_ONE, copy::DRIVERS_MANY]),
            certs: any_line(&receipt, Level::Bad, &[copy::CERT_ADDED]),
            checked,
            left: !left_lines.is_empty(),
            left_bytes: (!sizes.is_empty()).then(|| sizes.iter().sum()),
            name,
            id,
        });
        metas.push(meta);
    }
    if apps.is_empty() {
        return Err(format!("no results with a receipt in {}", results_dir.display()));
    }
    apps.sort_by(|a, b| (a.name.to_lowercase(), &a.id).cmp(&(b.name.to_lowercase(), &b.id)));
    not_installed.sort();

    let total = apps.len();
    let distinct = |f: fn(&Meta) -> &str| -> Vec<String> {
        metas
            .iter()
            .map(f)
            .filter(|s| !s.is_empty())
            .map(md_escape)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    };
    let dates = distinct(|m| &m.date);
    let date = match (dates.first(), dates.last()) {
        (Some(a), Some(b)) if a != b => format!("{a} to {b}"),
        (Some(a), _) => a.clone(),
        _ => "an unknown date".to_string(),
    };
    let runs: Vec<String> = metas
        .iter()
        .filter_map(|m| safe_run_url(&m.run_url))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|u| format!("[Workflow run]({u})"))
        .collect();
    let join_or = |v: Vec<String>, empty: &str| if v.is_empty() { empty.to_string() } else { v.join(", ") };

    let mut out = String::from("# Findings\n\n");
    out.push_str(&format!(
        "What {} did to a PC when installed, and what was left after uninstalling them. \
Each count is the number of apps with at least one matching line in their receipt; \
each app links to its receipt page.\n\n",
        plural(total, "popular Windows app", "popular Windows apps"),
    ));
    let list = |pick: &dyn Fn(&App) -> bool, of: &dyn Fn(&App) -> bool| -> (usize, usize, String) {
        let hits: Vec<String> = apps
            .iter()
            .filter(|a| of(a) && pick(a))
            .map(|a| format!("[{}]({})", md_escape(&a.name), a.link))
            .collect();
        let base = apps.iter().filter(|a| of(a)).count();
        (hits.len(), base, join_or(hits, "none"))
    };
    let all = |_: &App| true;
    out.push_str("## What the apps added\n\n");
    type Row<'a> = (&'a str, &'a dyn Fn(&App) -> bool);
    let rows: [Row; 7] = [
        ("added a background service", &|a| a.services),
        ("added a scheduled task", &|a| a.tasks),
        ("added a scheduled task that runs every day", &|a| a.daily_tasks),
        ("added a startup entry", &|a| a.startup),
        ("allowed incoming connections through the firewall", &|a| a.inbound),
        ("installed a driver", &|a| a.drivers),
        ("added a trusted root certificate", &|a| a.certs),
    ];
    for (what, pick) in rows {
        let (n, of, apps) = list(pick, &all);
        out.push_str(&format!("- **{n} of {of}** {what}: {apps}\n"));
    }

    out.push_str("\n## After uninstall\n\n");
    let checked = |a: &App| a.checked;
    let (n, of, apps_left) = list(&|a| a.left, &checked);
    out.push_str(&format!("- **{n} of {of}** left something behind: {apps_left}\n"));
    let mut sizes: Vec<u64> = apps.iter().filter(|a| a.checked).filter_map(|a| a.left_bytes).collect();
    sizes.sort_unstable();
    if !sizes.is_empty() {
        let mid = sizes.len() / 2;
        let median = if sizes.len() % 2 == 1 {
            sizes[mid]
        } else {
            (sizes[mid - 1] + sizes[mid]) / 2
        };
        out.push_str(&format!(
            "- The median amount of folder contents left behind is about {}, across the {} that left a folder with a size. \
Each app's amount adds up the sizes in its leftover folder lines, which are rounded.\n",
            format_size(median),
            plural(sizes.len(), "app", "apps"),
        ));
    }
    let unchecked: Vec<String> = apps
        .iter()
        .filter(|a| !a.checked)
        .map(|a| format!("[{}]({})", md_escape(&a.name), a.link))
        .collect();
    out.push_str(&format!(
        "- **{} of {total}** could not be checked after uninstall: {}\n",
        unchecked.len(),
        join_or(unchecked.clone(), "none"),
    ));
    if !unchecked.is_empty() {
        out.push_str("  The uninstall didn't finish on the test machine, so their leftovers are not counted above.\n");
    }

    if !not_installed.is_empty() {
        out.push_str(&format!(
            "\n{} did not finish installing, so {} not counted: {}\n",
            plural(not_installed.len(), "app", "apps"),
            if not_installed.len() == 1 { "it is" } else { "they are" },
            not_installed
                .iter()
                .map(|(n, _)| md_escape(n))
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }

    out.push_str("\n## How this was made\n\n");
    out.push_str(&format!(
        "Installed with winget on a fresh GitHub Actions runner ({}) by idiff {} on {date}, opened once, then uninstalled.{} \
idiff only reads; see the README for how it decides what belongs to the app.\n",
        join_or(distinct(|m| &m.runner_image), "unknown image"),
        join_or(distinct(|m| &m.idiff_version), "unknown version"),
        if runs.is_empty() { String::new() } else { format!(" {}.", runs.join(", ")) },
    ));
    out.push_str(
        "\nWhat counts: background services, scheduled tasks, drivers and inbound firewall rules are the \
receipt's own warning lines of those names. A startup entry is the \"Starts with Windows\" line; a service that \
starts with Windows is counted as a service only. A scheduled task counts as daily when its line says it runs every day. \
Root certificates are the \"Adds a trusted root certificate\" lines. Leftovers are the lines under \"After uninstall\" \
that say something is still there.\n",
    );
    Ok(out)
}
