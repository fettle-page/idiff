//! The noise list: changes Windows makes by itself, which are never shown as the app's (R1).
//! Built in from `noise.toml`; a local `noise.local.toml` in the same format adds to it.

use crate::copy;
use crate::diff::Change;
use crate::model::{Item, SourceKind};
use crate::publisher;
use crate::subject::Subject;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct NoiseList {
    rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
struct Rule {
    source: SourceKind,
    key: Option<Regex>,
    target: Option<Regex>,
    props: Vec<(String, Regex)>,
    unless_publisher: Option<String>,
    /// Set: the rule only matches a modified item whose changed fields are all in this list.
    only_changed: Option<Vec<String>>,
    note: String,
}

/// One `[[rule]]` as written in the file.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleText {
    source: SourceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    props: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unless_publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    only_changed: Option<Vec<String>>,
    note: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileText {
    #[serde(default, rename = "rule")]
    rules: Vec<RuleText>,
}

fn regex(pattern: &str) -> Result<Regex, regex::Error> {
    RegexBuilder::new(pattern).case_insensitive(true).build()
}

impl NoiseList {
    pub fn builtin() -> NoiseList {
        NoiseList::parse(include_str!("../noise.toml")).expect("the built-in noise list parses")
    }

    pub fn parse(text: &str) -> Result<NoiseList, String> {
        let file: FileText = toml::from_str(text).map_err(|e| e.to_string())?;
        let rules = file
            .rules
            .into_iter()
            .enumerate()
            .map(|(n, r)| {
                let at = |e: String| format!("rule {} ({:?}): {e}", n + 1, r.source);
                if r.key.is_none() && r.target.is_none() && r.props.is_empty() {
                    return Err(at("needs a key, target or props pattern".into()));
                }
                let compile = |p: &String| regex(p).map_err(|e| at(e.to_string()));
                Ok(Rule {
                    source: r.source,
                    key: r.key.as_ref().map(compile).transpose()?,
                    target: r.target.as_ref().map(compile).transpose()?,
                    props: r
                        .props
                        .iter()
                        .map(|(name, p)| Ok((name.clone(), compile(p)?)))
                        .collect::<Result<_, String>>()?,
                    unless_publisher: r.unless_publisher,
                    only_changed: r.only_changed,
                    note: r.note,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(NoiseList { rules })
    }

    /// The built-in rules first, then `other`'s.
    pub fn merged(mut self, other: NoiseList) -> NoiseList {
        self.rules.extend(other.rules);
        self
    }

    /// The note of the first rule that matches `item`. Rules about how an item changed
    /// (`only_changed`) need the whole change: see `matches_change`.
    pub fn matches(&self, kind: SourceKind, item: &Item, subject: &Subject) -> Option<&str> {
        self.rules
            .iter()
            .find(|r| r.only_changed.is_none() && r.applies(kind, item, subject))
            .map(|r| r.note.as_str())
    }

    /// The note of the first rule that matches this change.
    pub fn matches_change(&self, kind: SourceKind, change: &Change, subject: &Subject) -> Option<&str> {
        self.rules
            .iter()
            .find(|r| {
                let changed_ok = match (&r.only_changed, change) {
                    (None, _) => true,
                    (Some(allowed), Change::Modified { fields, .. }) => fields.iter().all(|f| allowed.contains(f)),
                    (Some(_), _) => false,
                };
                changed_ok && r.applies(kind, change.item(), subject)
            })
            .map(|r| r.note.as_str())
    }
}

impl Rule {
    fn applies(&self, kind: SourceKind, item: &Item, subject: &Subject) -> bool {
        let unless = self
            .unless_publisher
            .as_deref()
            .zip(subject.publisher.as_deref())
            .is_some_and(|(rule, app)| publisher::same(rule, app));
        self.source == kind
            && !unless
            && self.key.as_ref().is_none_or(|k| k.is_match(&item.key))
            && self
                .target
                .as_ref()
                .is_none_or(|t| item.target.as_deref().is_some_and(|v| t.is_match(v)))
            && self
                .props
                .iter()
                .all(|(name, p)| item.props.get(name).is_some_and(|v| p.is_match(v)))
    }
}

/// Rules that match exactly these keys, for `idiff calibrate --save`.
pub fn exact_rules(keys: &[(SourceKind, String)]) -> String {
    let file = FileText {
        rules: keys
            .iter()
            .map(|(source, key)| RuleText {
                source: *source,
                key: Some(format!("^{}$", regex::escape(key))),
                target: None,
                props: BTreeMap::new(),
                unless_publisher: None,
                only_changed: None,
                note: copy::NOTE_CALIBRATE.to_string(),
            })
            .collect(),
    };
    toml::to_string(&file).expect("noise rules always serialize")
}
