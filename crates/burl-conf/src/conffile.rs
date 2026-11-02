//! `ConfFile`, `ConfSection` and `ConfEntry` (Core `conffile.cpp`, pinned submodule commit
//! d8d880f). Values stay raw strings until `settings` imports them. Sections and entries
//! are kept in the C++ `std::map` orders so `save` output matches: sections by name (repeats
//! in insertion order), entries by key (repeats in insertion order).
//!
//! Values are `String`, so a command-line argument or config line that is not valid UTF-8 is
//! converted lossily (a recorded Gap for non-UTF-8 paths).

use burl_util::regex::Regex;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::LazyLock;

/// A `ConfFileException`: the message is final (file and line prefix already applied).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfError(pub String);

impl fmt::Display for ConfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfError {}

static SECTION: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^\s*\[([^\]]+)\]\s*$"));
static KEY_VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::must(r"^\s*([^=\s]+)\s*=\s*([^\s].*)?$"));
static VEC3: LazyLock<Regex> = LazyLock::new(|| {
    Regex::must(r"^\s*vec3\(\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*\)\s*$")
});

/// `atof`: the longest decimal (or inf/nan) prefix after leading whitespace, 0.0 if none.
pub fn atof(s: &str) -> f64 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let start = i;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let rest = &s[i..];
    let lower = rest.get(..8).unwrap_or(rest).to_ascii_lowercase();
    for word in ["infinity", "inf", "nan"] {
        if lower.starts_with(word) {
            let v = if word == "nan" {
                f64::NAN
            } else {
                f64::INFINITY
            };
            return if b[start] == b'-' { -v } else { v };
        }
    }
    let digits = |mut j: usize| {
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        j
    };
    let int_end = digits(i);
    let mut end = int_end;
    let mut any = int_end > i;
    if end < b.len() && b[end] == b'.' {
        let frac_end = digits(end + 1);
        any |= frac_end > end + 1;
        end = frac_end;
    }
    if !any {
        return 0.0;
    }
    if end < b.len() && (b[end] == b'e' || b[end] == b'E') {
        let mut j = end + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let exp_end = digits(j);
        if exp_end > j {
            end = exp_end;
        }
    }
    s[start..end].parse().unwrap_or(0.0)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfEntry {
    pub key: String,
    pub value: String,
    /// 1-based line in the config file, 0 for entries that came from the command line.
    pub line: usize,
}

impl ConfEntry {
    pub fn has_value(&self) -> bool {
        !self.value.is_empty()
    }
    pub fn get_int(&self) -> i32 {
        burl_util::cstr::atoi(self.value.as_bytes())
    }
    pub fn get_float(&self) -> f32 {
        atof(&self.value) as f32
    }
    /// True only for `1 true True TRUE yes Yes YES`; everything else is false.
    pub fn get_bool(&self) -> bool {
        matches!(
            self.value.as_str(),
            "1" | "true" | "True" | "TRUE" | "yes" | "Yes" | "YES"
        )
    }
    pub fn is_vec3(&self) -> bool {
        VEC3.is_match(self.value.as_bytes())
    }
    pub fn get_vec3(&self) -> [f32; 3] {
        match VEC3.captures(self.value.as_bytes()) {
            Some(m) => {
                let f = |i: usize| atof(&String::from_utf8_lossy(&m[i])) as f32;
                [f(0), f(1), f(2)]
            }
            None => [0.0; 3],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfSection {
    name: String,
    line: usize,
    entries: BTreeMap<String, Vec<ConfEntry>>,
}

impl ConfSection {
    pub fn new(name: &str, line: usize) -> Self {
        ConfSection {
            name: name.to_string(),
            line,
            entries: BTreeMap::new(),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn entries(&self, key: &str) -> &[ConfEntry] {
        self.entries.get(key).map(Vec::as_slice).unwrap_or(&[])
    }
    /// The first entry for `key` (what single-valued settings read).
    pub fn entry(&self, key: &str) -> Option<&ConfEntry> {
        self.entries(key).first()
    }
    pub fn add_entry(&mut self, key: &str, value: &str, line: usize) {
        let e = ConfEntry {
            key: key.to_string(),
            value: value.to_string(),
            line,
        };
        self.entries.entry(key.to_string()).or_default().push(e);
    }
    /// Replaces every entry of `key` with one.
    pub fn set_entry(&mut self, key: &str, value: &str) {
        let e = ConfEntry {
            key: key.to_string(),
            value: value.to_string(),
            line: 0,
        };
        self.entries.insert(key.to_string(), vec![e]);
    }
    pub fn get_bool(&self, key: &str) -> bool {
        self.entry(key).is_some_and(ConfEntry::get_bool)
    }
    pub fn has_value(&self, key: &str) -> bool {
        self.entry(key).is_some_and(ConfEntry::has_value)
    }
    pub fn get_string(&self, key: &str) -> &str {
        self.entry(key).map_or("", |e| e.value.as_str())
    }
    fn print(&self, out: &mut String) {
        out.push_str(&format!("[{}]\n", self.name));
        for list in self.entries.values() {
            for e in list {
                out.push_str(&format!("{}={}\n", e.key, e.value));
            }
        }
        out.push('\n');
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfFile {
    filename: String,
    sections: BTreeMap<String, Vec<ConfSection>>,
}

impl ConfFile {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn clear(&mut self) {
        self.sections.clear();
    }
    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn load(&mut self, filename: &str) -> Result<(), ConfError> {
        self.filename = filename.to_string();
        self.clear();
        let data = std::fs::read(filename)
            .map_err(|_| ConfError(format!("failed to open config file {filename}")))?;
        self.parse(&data)
    }

    /// The `ConfFile::load` grammar over file contents; `filename` only feeds error messages.
    pub fn parse(&mut self, data: &[u8]) -> Result<(), ConfError> {
        let mut lines: Vec<&[u8]> = data.split(|b| *b == b'\n').collect();
        if data.last() == Some(&b'\n') || data.is_empty() {
            lines.pop(); // std::getline yields no empty final line
        }
        let mut sec: Option<ConfSection> = None;
        for (i, line) in lines.iter().enumerate() {
            let lineno = i + 1;
            if line.is_empty() || line[0] == b'#' {
                continue;
            }
            if let Some(m) = SECTION.captures(line) {
                if let Some(s) = sec.take() {
                    self.push_section(s);
                }
                sec = Some(ConfSection::new(&String::from_utf8_lossy(&m[0]), lineno));
            } else if let Some(m) = KEY_VALUE.captures(line) {
                let key = String::from_utf8_lossy(&m[0]).into_owned();
                let value = m
                    .get(1)
                    .map(|v| String::from_utf8_lossy(v).into_owned())
                    .unwrap_or_default();
                let value = value.trim_end_matches([' ', '\t', '\x0c', '\x0b', '\n', '\r']);
                sec.get_or_insert_with(|| ConfSection::new("", lineno))
                    .add_entry(&key, value, lineno);
            } else {
                return Err(ConfError(format!(
                    "{}, line {}: could not parse line",
                    self.filename, lineno
                )));
            }
        }
        if let Some(s) = sec {
            self.push_section(s);
        }
        Ok(())
    }

    /// `ConfFile::save`: sections in name order, entries in key order.
    pub fn save(&mut self, filename: &str) -> Result<(), ConfError> {
        self.filename = filename.to_string();
        let mut out = String::new();
        for list in self.sections.values() {
            for s in list {
                s.print(&mut out);
            }
        }
        std::fs::write(filename, out)
            .map_err(|_| ConfError(format!("failed to write config to {filename}")))
    }

    fn push_section(&mut self, s: ConfSection) {
        self.sections.entry(s.name.clone()).or_default().push(s);
    }
    pub fn add_section(&mut self, name: &str) -> &mut ConfSection {
        let list = self.sections.entry(name.to_string()).or_default();
        list.push(ConfSection::new(name, 0));
        list.last_mut().unwrap()
    }
    pub fn sections(&self, name: &str) -> &[ConfSection] {
        self.sections.get(name).map(Vec::as_slice).unwrap_or(&[])
    }
    pub fn sections_mut(&mut self, name: &str) -> &mut [ConfSection] {
        self.sections
            .get_mut(name)
            .map(Vec::as_mut_slice)
            .unwrap_or(&mut [])
    }
    pub fn count_section(&self, name: &str) -> usize {
        self.sections(name).len()
    }
    /// The first section with that name.
    pub fn section(&self, name: &str) -> Option<&ConfSection> {
        self.sections(name).first()
    }
    pub fn section_mut(&mut self, name: &str) -> Option<&mut ConfSection> {
        self.sections_mut(name).first_mut()
    }
    /// `setEntry` on the first section of that name, creating it if needed.
    pub fn set_entry(&mut self, section: &str, key: &str, value: &str) {
        if self.section(section).is_none() {
            self.add_section(section);
        }
        self.section_mut(section).unwrap().set_entry(key, value);
    }

    /// `entryException`: `<file>[, line N]: <reason>`, the prefix only if a file is set.
    pub fn entry_error(&self, entry: &ConfEntry, reason: &str) -> ConfError {
        let mut msg = String::new();
        if !self.filename.is_empty() {
            msg.push_str(&self.filename);
            if entry.line != 0 {
                msg.push_str(&format!(", line {}", entry.line));
            }
            msg.push_str(": ");
        }
        msg.push_str(reason);
        ConfError(msg)
    }
    pub fn missing_value_error(&self, entry: &ConfEntry) -> ConfError {
        self.entry_error(entry, &format!("no value specified for '{}'", entry.key))
    }
    pub fn invalid_value_error(&self, entry: &ConfEntry) -> ConfError {
        self.entry_error(entry, &format!("invalid '{}' value", entry.key))
    }
}
