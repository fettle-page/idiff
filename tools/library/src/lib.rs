//! Turns catalog results into the Receipts Library: a Markdown page per app version, a page per
//! app, and the index (`receipts/README.md` and `receipts/index.json`).
//!
//! Pages use only the receipts' own words plus the fixed text below: no rankings, scores or
//! words about vendors. Every page says how it was made.

pub mod findings;

use idiff_core::copy;
use idiff_core::render::{markdown_section, md_escape};
use idiff_core::verdict::{Level, Receipt, hidden_count_text};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// What the catalog runner script writes next to each app's receipts.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Meta {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub idiff_version: String,
    #[serde(default)]
    pub runner_image: String,
    #[serde(default)]
    pub run_url: String,
    /// Set when a step failed: `"<step>: exit <n>"`.
    #[serde(default)]
    pub failed: Option<String>,
}

/// One row of `receipts/index.json` (schema v1).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub date: String,
    pub warn: u32,
    pub bad: u32,
    pub leftovers: u32,
    /// False when the uninstall didn't finish, so `leftovers` (then 0) means nothing. Added to
    /// schema v1; older files without it read as checked.
    #[serde(default = "checked")]
    pub leftovers_checked: bool,
    pub changed: bool,
    pub path: String,
}

fn checked() -> bool {
    true
}

/// What one version's `.json` file in the library holds.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Record {
    meta: Meta,
    receipt: Receipt,
    leftovers: Option<Receipt>,
}

/// What a run of the tool did, for the pull request's description.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    /// Winget ids with a new version page.
    pub added: Vec<String>,
    /// Winget ids whose version already has a page.
    pub skipped_same: Vec<String>,
    /// Winget id and why; their previous entry stays.
    pub failed: Vec<(String, String)>,
    /// Winget ids in `added` whose uninstall didn't finish, so their leftovers aren't checked.
    pub leftovers_unchecked: Vec<String>,
}

const INTRO: &str = "What popular Windows apps do to your PC when you install them, and what they leave behind. \
Every page is made automatically by idiff, the same way you can run it yourself.";
const HOW_MADE: &str = "Installed with winget on a fresh GitHub Actions runner ({runner_image}) by idiff \
{idiff_version} on {date}, opened once, then uninstalled.{run} idiff only reads; see the README for how it \
decides what belongs to the app.";
/// From a version page in `receipts/<app>/` to the README's section on attribution.
const HOW_IT_DECIDES: &str = "[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)";
const NO_META: &str = "no meta.json";
/// Under "After uninstall" when the uninstall step failed but the install receipt is good.
const UNINSTALL_UNFINISHED: &str =
    "The uninstall didn't finish on the test machine, so these may include things it would have removed.";
/// The index's Leftovers cell for those.
const NOT_CHECKED: &str = "not checked";
const NO_RECEIPT: &str = "no receipt.json (the run didn't finish)";

/// A file name for a version: lowercase `[a-z0-9.-]`, anything else `-`, no dots or dashes at
/// either end (no hidden files, nothing Windows trims), `unknown` if nothing is left.
pub fn version_slug(v: &str) -> String {
    let s: String = v
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '.' | '-' => c,
            _ => '-',
        })
        .collect();
    let s = s.trim_matches(['.', '-']);
    if s.is_empty() { "unknown".into() } else { s.into() }
}

/// An app's folder name from its winget id: `version_slug`, with `+` spelled out first, so
/// `Notepad++.Notepad++` is `notepadplusplus.notepadplusplus` rather than `notepad--.notepad`.
pub fn id_slug(id: &str) -> String {
    version_slug(&id.replace('+', "plus"))
}

/// Lines that come from the run rather than the app: how much Windows did meanwhile, what
/// couldn't be read, how the command exited. They'd make every version look "changed".
fn is_finding(text: &str) -> bool {
    let before = |template: &'static str, part: &str| template.split(part).next().unwrap_or(template);
    let after = |template: &'static str, part: &str| template.split(part).nth(1).unwrap_or(template);
    !(text == copy::HIDDEN_ONE
        || text.ends_with(after(copy::HIDDEN_MANY, "{n}"))
        || text.starts_with(before(copy::COULDNT_CHECK, "{source}"))
        || text.starts_with(before(copy::ONLY_PARTLY_CHECKED, "{source}"))
        || text.starts_with(before(copy::EXIT_CODE, "{n}"))
        || text.starts_with(before(copy::RUN_TIMED_OUT, "{n}")))
}

/// Findings in `cur` that weren't in `prev` (`+ `), then the reverse (`- `).
pub fn changes_since(prev: &Receipt, cur: &Receipt) -> Vec<String> {
    let texts = |r: &Receipt| -> Vec<String> {
        r.lines
            .iter()
            .map(|l| l.text.clone())
            .filter(|t| is_finding(t))
            .collect()
    };
    let (prev, cur) = (texts(prev), texts(cur));
    let added = cur.iter().filter(|t| !prev.contains(t)).map(|t| format!("+ {t}"));
    let removed = prev.iter().filter(|t| !cur.contains(t)).map(|t| format!("- {t}"));
    added.chain(removed).collect()
}

/// Only links to the project's own workflow runs, so a bad URL can't become a link.
fn safe_run_url(url: &str) -> Option<&str> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || "/._-:".contains(c);
    (url.starts_with("https://github.com/") && url.chars().all(allowed)).then_some(url)
}

/// A library reader has no session for `idiff show --all`, so pages list the hidden changes and
/// the count line says so. Only the rendered page changes; the stored receipt stays as idiff made it.
fn listing_hidden(receipt: &Receipt) -> Receipt {
    let mut shown = receipt.clone();
    let (cli, page) = (
        hidden_count_text(receipt.hidden.len(), false),
        hidden_count_text(receipt.hidden.len(), true),
    );
    for line in &mut shown.lines {
        if line.text == cli {
            line.text.clone_from(&page);
        }
    }
    shown
}

pub fn version_page(meta: &Meta, receipt: &Receipt, leftovers: Option<&Receipt>, changes: &[String]) -> String {
    let mut page = format!(
        "# {} {}\n\n{}",
        md_escape(&meta.name),
        md_escape(&meta.version),
        markdown_section(&listing_hidden(receipt), true, 2)
    );
    let unfinished = uninstall_unfinished(meta);
    if leftovers.is_some() || unfinished {
        page.push_str("\n## After uninstall\n\n");
        if unfinished {
            page.push_str(&format!("{UNINSTALL_UNFINISHED}\n\n"));
        }
        if let Some(leftovers) = leftovers {
            page.push_str(&markdown_section(&listing_hidden(leftovers), true, 3));
        }
    }
    if !changes.is_empty() {
        page.push_str("\n## Changed since the previous version\n\n");
        for change in changes {
            // `\+` and `\-`: a bare + or - after the bullet would start a nested list.
            let (sign, text) = change.split_at(1.min(change.len()));
            page.push_str(&format!("- \\{sign}{}\n", md_escape(text)));
        }
    }
    let run = safe_run_url(&meta.run_url)
        .map(|url| format!(" [Workflow run]({url})."))
        .unwrap_or_default();
    let how = HOW_MADE
        .replace("{runner_image}", &md_escape(&meta.runner_image))
        .replace("{idiff_version}", &md_escape(&meta.idiff_version))
        .replace("{date}", &md_escape(&meta.date))
        .replace("{run}", &run);
    page.push_str(&format!("\n## How this was made\n\n{how}\n\n{HOW_IT_DECIDES}\n"));
    page
}

/// Keeps a path from `index.json` to the characters the tool itself writes.
fn safe_path(path: &str) -> String {
    path.chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "./-".contains(*c))
        .collect()
}

pub fn index_md(entries: &[Entry]) -> String {
    let mut out = format!(
        "# Receipts Library\n\n{INTRO}\n\n| App | Version | ⚠ | ✗ | Leftovers | Changed | Checked |\n|---|---|---:|---:|---:|---|---|\n"
    );
    for e in entries {
        out.push_str(&format!(
            "| [{}]({}) | {} | {} | {} | {} | {} | {} |\n",
            md_escape(&e.name),
            safe_path(&e.path),
            md_escape(&e.version),
            e.warn,
            e.bad,
            if e.leftovers_checked {
                e.leftovers.to_string()
            } else {
                NOT_CHECKED.to_string()
            },
            if e.changed { "yes" } else { "" },
            md_escape(&e.date),
        ));
    }
    out
}

impl Summary {
    pub fn to_markdown(&self) -> String {
        summary_markdown(self, &[])
    }
}

/// The summary, naming added apps by name and version where `entries` has them.
pub fn summary_markdown(s: &Summary, entries: &[Entry]) -> String {
    let mut out = String::from("## Receipts Library update\n\n");
    if s.added.is_empty() && s.skipped_same.is_empty() && s.failed.is_empty() {
        out.push_str("No results to add.\n");
    }
    if !s.added.is_empty() {
        out.push_str("Added:\n\n");
        for id in &s.added {
            let line = match entries.iter().find(|e| &e.id == id) {
                Some(e) => format!(
                    "{} {} ({}){}",
                    md_escape(&e.name),
                    md_escape(&e.version),
                    md_escape(id),
                    if e.changed {
                        ", changed since the previous version"
                    } else {
                        ""
                    }
                ),
                None => md_escape(id),
            };
            out.push_str(&format!("- {line}\n"));
        }
        out.push('\n');
    }
    if !s.skipped_same.is_empty() {
        out.push_str("Same version as before, so not rewritten:\n\n");
        for id in &s.skipped_same {
            out.push_str(&format!("- {}\n", md_escape(id)));
        }
        out.push('\n');
    }
    if !s.leftovers_unchecked.is_empty() {
        out.push_str("Added without a leftovers check, because the uninstall didn't finish:\n\n");
        for id in &s.leftovers_unchecked {
            out.push_str(&format!("- {}\n", md_escape(id)));
        }
        out.push('\n');
    }
    if !s.failed.is_empty() {
        out.push_str("Failed, so the previous page stays:\n\n");
        for (id, why) in &s.failed {
            out.push_str(&format!("- {}: {}\n", md_escape(id), md_escape(why)));
        }
    }
    out
}

/// Writes only when the content is new, so an unchanged library stays byte for byte (and
/// file times) the same.
fn write_if_changed(path: &Path, content: &str) -> Result<(), String> {
    if fs::read_to_string(path).is_ok_and(|old| old == content) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, content).map_err(|e| format!("{}: {e}", path.display()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>, String> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| format!("{}: {e}", path.file_name().unwrap_or_default().to_string_lossy())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Every version already in an app's folder, newest first, with its file stem.
fn records(app_dir: &Path) -> Vec<(String, Record)> {
    let Ok(entries) = fs::read_dir(app_dir) else {
        return Vec::new();
    };
    let mut found: Vec<(String, Record)> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| {
            let stem = p.file_stem()?.to_string_lossy().into_owned();
            Some((stem, read_json::<Record>(&p).ok()??))
        })
        .collect();
    found.sort_by(|a, b| (&b.1.meta.date, &b.0).cmp(&(&a.1.meta.date, &a.0)));
    found
}

fn count(r: &Receipt, level: Level) -> u32 {
    r.lines.iter().filter(|l| l.level == level).count() as u32
}

/// The catalog script names the step that failed. When only the uninstall check failed (most
/// uninstallers wait on a dialog on a runner), the install receipt is still good.
fn uninstall_unfinished(meta: &Meta) -> bool {
    meta.failed.as_deref().is_some_and(|f| f.starts_with("leftovers:"))
}

/// One app's result into the library: `Some(leftovers unchecked)` when a page was written, `None`
/// when that version was there already. Errors are the reasons it's listed as failed.
fn apply_one(
    dir: &Path,
    receipts: &Path,
    entries: &mut Vec<Entry>,
) -> Result<(String, Option<bool>), (String, String)> {
    let dir_name = dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let meta: Meta = read_json(&dir.join("meta.json"))
        .map_err(|e| (dir_name.clone(), e))?
        .ok_or_else(|| (dir_name.clone(), NO_META.to_string()))?;
    let id = if meta.id.is_empty() { dir_name } else { meta.id.clone() };
    let fail = |why: String| (id.clone(), why);
    let unfinished = uninstall_unfinished(&meta);
    if let Some(why) = meta.failed.as_ref().filter(|_| !unfinished) {
        return Err(fail(why.clone()));
    }
    // A hung or cancelled job can leave only meta.json.
    let receipt: Receipt = read_json(&dir.join("receipt.json"))
        .map_err(fail)?
        .ok_or_else(|| fail(meta.failed.clone().unwrap_or_else(|| NO_RECEIPT.to_string())))?;
    let leftovers: Option<Receipt> = read_json(&dir.join("leftovers.json")).map_err(fail)?;

    let app_dir = receipts.join(id_slug(&id));
    let mut existing = records(&app_dir);
    // The same version is skipped, unless this run checked the leftovers that one couldn't.
    let mut replacing = None;
    if let Some((stem, same)) = existing.iter().find(|(_, r)| r.meta.version == meta.version) {
        if !uninstall_unfinished(&same.meta) || unfinished {
            return Ok((id, None));
        }
        replacing = Some(stem.clone());
    }
    existing.retain(|(stem, _)| Some(stem) != replacing.as_ref());
    // Two versions can share a slug ("1.0+a", "1.0/a"): the later one gets a number.
    let base = version_slug(&meta.version);
    let slug = replacing.unwrap_or_else(|| {
        std::iter::once(base.clone())
            .chain((2..).map(|n| format!("{base}-{n}")))
            .find(|s| !app_dir.join(format!("{s}.json")).exists())
            .expect("some numbered name is free")
    });

    let previous = existing.first().map(|(_, r)| r);
    let changes = previous
        .map(|p| changes_since(&p.receipt, &receipt))
        .unwrap_or_default();
    let page = version_page(&meta, &receipt, leftovers.as_ref(), &changes);
    let record = Record {
        meta: meta.clone(),
        receipt: receipt.clone(),
        leftovers: leftovers.clone(),
    };
    let json = serde_json::to_string_pretty(&record).expect("a record always serializes") + "\n";
    write_if_changed(&app_dir.join(format!("{slug}.json")), &json).map_err(fail)?;
    write_if_changed(&app_dir.join(format!("{slug}.md")), &page).map_err(fail)?;

    // The app's own page: the newest version, then links to the older ones.
    let mut app_page = page.clone();
    if !existing.is_empty() {
        app_page.push_str("\n## Older versions\n\n");
        for (stem, r) in &existing {
            app_page.push_str(&format!(
                "- [{}]({stem}.md) ({})\n",
                md_escape(&r.meta.version),
                md_escape(&r.meta.date)
            ));
        }
    }
    write_if_changed(&app_dir.join("README.md"), &app_page).map_err(fail)?;

    entries.retain(|e| e.id != id);
    entries.push(Entry {
        id: id.clone(),
        name: meta.name.clone(),
        version: meta.version.clone(),
        date: meta.date.clone(),
        warn: count(&receipt, Level::Warn),
        bad: count(&receipt, Level::Bad),
        leftovers: match unfinished {
            true => 0,
            false => leftovers.as_ref().map_or(0, |l| count(l, Level::Bad)),
        },
        leftovers_checked: !unfinished,
        changed: !changes.is_empty(),
        path: format!("{}/{slug}.md", id_slug(&id)),
    });
    Ok((id, Some(unfinished)))
}

/// Adds every app in `results_dir` (`<id>/meta.json`, `receipt.json`, `leftovers.json`) to the
/// library in `receipts_dir`. A failed app, or one whose version is already there, keeps its
/// entry as it was.
pub fn apply(results_dir: &Path, receipts_dir: &Path) -> Result<Summary, String> {
    let index_path = receipts_dir.join("index.json");
    let mut entries: Vec<Entry> = read_json(&index_path)?.unwrap_or_default();
    let mut dirs: Vec<PathBuf> = fs::read_dir(results_dir)
        .map_err(|e| format!("{}: {e}", results_dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();

    let mut summary = Summary::default();
    for dir in &dirs {
        match apply_one(dir, receipts_dir, &mut entries) {
            Ok((id, Some(unchecked))) => {
                if unchecked {
                    summary.leftovers_unchecked.push(id.clone());
                }
                summary.added.push(id);
            }
            Ok((id, None)) => summary.skipped_same.push(id),
            Err(failed) => summary.failed.push(failed),
        }
    }

    entries.sort_by(|a, b| (a.name.to_lowercase(), &a.id).cmp(&(b.name.to_lowercase(), &b.id)));
    let json = serde_json::to_string_pretty(&entries).expect("entries always serialize") + "\n";
    write_if_changed(&index_path, &json)?;
    write_if_changed(&receipts_dir.join("README.md"), &index_md(&entries))?;
    Ok(summary)
}

/// For the binary: the summary with names and versions from the index just written.
pub fn summary_for(receipts_dir: &Path, summary: &Summary) -> String {
    let entries: Vec<Entry> = read_json(&receipts_dir.join("index.json"))
        .ok()
        .flatten()
        .unwrap_or_default();
    summary_markdown(summary, &entries)
}
