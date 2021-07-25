use std::sync::Arc;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::{bookmarks::Bookmark, storage::friendly_path};

pub fn find_matches(
    matcher: &SkimMatcherV2,
    bookmarks: &[Arc<Bookmark>],
    pattern: String,
) -> Vec<usize> {
    // Rank all bookmarks using fuzzy matcher
    let mut scores: Vec<_> = bookmarks
        .iter()
        .map(|bm| {
            matcher.fuzzy_match(
                &format!("{} {}", bm.name, friendly_path(&bm.dest)),
                &pattern,
            )
        })
        .enumerate()
        .collect();
    // Reverse sort the scores
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Pick the matches starting from the "best" one
    let mut matches = Vec::new();
    for (idx, score) in &scores {
        if let Some(score) = *score {
            if score > 0 {
                matches.push(*idx);
            }
        }
    }

    matches
}
