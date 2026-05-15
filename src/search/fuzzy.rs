use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

use crate::discovery::types::AppEntry;

pub fn fuzzy_match<'a>(entries: &'a [AppEntry], query: &str) -> Vec<&'a AppEntry> {
    if query.is_empty() {
        return entries.iter().take(20).collect();
    }

    let mut matcher = Matcher::default();
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);

    let mut scored: Vec<(u32, &AppEntry)> = entries
        .iter()
        .filter_map(|entry| {
            let score = pattern.score(Utf32Str::Ascii(&entry.name_lower.as_bytes()), &mut matcher)?;
            Some((score, entry))
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().take(20).map(|(_, e)| e).collect()
}
