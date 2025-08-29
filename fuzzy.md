# Fuzzy Matching Guide - LST Implementation

## Overview

LST uses a multi-tier fuzzy matching system that combines fuzzy string matching with substring fallback for robust text search across lists and notes.

## Dependencies

Add to your `Cargo.toml`:

```toml
fuzzy-matcher = "0.3"
```

## Core Implementation

**Enhanced fuzzy matching with scoring and ranking:**

```rust
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

/// Find items by fuzzy matching text with scoring and ranking
/// Returns a vector of matching indices sorted by relevance score
pub fn fuzzy_find(items: &[ListItem], query: &str, threshold: f32) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }

    let matcher = SkimMatcherV2::default();
    let mut matches_with_scores: Vec<(usize, i64)> = Vec::new();

    for (index, item) in items.iter().enumerate() {
        // Try fuzzy matching on the item text
        if let Some(score) = matcher.fuzzy_match(&item.text, query) {
            // Convert score to f32 for threshold comparison
            let score_f32 = score as f32;
            if score_f32 >= threshold {
                matches_with_scores.push((index, score));
            }
        }

        // Also try substring matching as fallback for very short queries
        if query.len() <= 3 && item.text.to_lowercase().contains(&query.to_lowercase()) {
            // Give substring matches a lower score boost
            let substring_score = (query.len() * 50) as i64;
            if !matches_with_scores.iter().any(|(idx, _)| *idx == index) {
                matches_with_scores.push((index, substring_score));
            }
        }
    }

    // Sort by score (highest first) and return indices
    matches_with_scores.sort_by(|a, b| b.1.cmp(&a.1));
    matches_with_scores.into_iter().map(|(idx, _)| idx).collect()
}
```

## Configuration Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyConfig {
    #[serde(default = "default_threshold")]
    pub threshold: f32,  // 50.0 default - minimum match score
    #[serde(default = "default_max_suggestions")]
    pub max_suggestions: usize,  // 7 default - max results to return
}
```

## Matching Strategy

**Multi-tier approach:**

1. **Primary**: Fuzzy string matching using SkimMatcherV2 (handles typos, partial matches)
2. **Fallback**: Substring matching for short queries (≤3 chars)
3. **Scoring**: Results ranked by relevance score
4. **Threshold**: Configurable minimum match quality

## Usage Patterns

**1. CLI Command Resolution:**
```rust
fn normalize_list(input: &str) -> Result<String> {
    let key = input.trim_end_matches(".md");

    // Try exact match first (fast path)
    for entry in &entries {
        if entry.name == key {
            return Ok(entry.relative_path.clone());
        }
    }

    // Then fuzzy match with scoring
    let matches: Vec<_> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            let matcher = SkimMatcherV2::default();
            matcher.fuzzy_match(&entry.name, key)
                .map(|score| (i, score))
        })
        .filter(|(_, score)| *score >= 50) // threshold
        .collect::<Vec<_>>();

    // Sort by score and take top matches
    matches.sort_by(|a, b| b.1.cmp(&a.1));
    let top_matches: Vec<_> = matches.into_iter()
        .take(7) // max_suggestions
        .map(|(i, _)| &entries[i])
        .collect();

    match top_matches.len() {
        0 => Ok(key.to_string()), // Allow new creation
        1 => Ok(top_matches[0].relative_path.clone()),
        _ => bail!("Multiple matches: {:?}", top_matches.iter().map(|e| &e.relative_path).collect::<Vec<_>>())
    }
}
```

**2. Item Finding in Lists:**
```rust
// Find items with fuzzy matching and ranking
let matches = fuzzy_find(&all_items, target, config.fuzzy.threshold);

// Limit to max suggestions
let suggestions: Vec<_> = matches.into_iter()
    .take(config.fuzzy.max_suggestions)
    .map(|idx| &all_items[idx])
    .collect();

// Example: Find "apple" in ["pineapple", "apple pie", "grape"]
// With threshold=50.0, all three would match (scored by relevance)
// With threshold=80.0, only "apple pie" might match (better matches only)
```

## Performance Characteristics

- **Fast**: SkimMatcherV2 is optimized for speed
- **Memory efficient**: No pre-processing required
- **Scalable**: Works well with large item lists
- **Configurable**: Threshold and max results tuning

## For Your CLI

**Quick Transfer:**
1. Add `fuzzy-matcher = "0.3"` to Cargo.toml
2. Copy the `fuzzy_find` function
3. Add `FuzzyConfig` to your config structure
4. Use the multi-tier matching approach

**Integration Tips:**
- Start with threshold 0.75, adjust based on your use case
- For CLI commands, limit suggestions to 5-10 items
- Consider caching matcher instances for repeated searches
- Add case-insensitive options if needed

This implementation provides robust fuzzy matching suitable for CLI tools, file search, and item selection workflows.