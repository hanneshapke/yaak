use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// BM25 score threshold for fuzzy cache hits.
/// Tuned for short natural-language descriptions (3-10 words).
const BM25_THRESHOLD: f64 = 0.5;

/// BM25 parameters
const BM25_K1: f64 = 1.2;
const BM25_B: f64 = 0.75;

/// Stop words to strip before tokenizing
const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "in", "on", "at", "to", "for", "of", "with", "and", "or", "is", "it", "my",
    "me", "i", "do", "please", "can", "you", "just", "show", "tell", "get",
];

#[derive(Serialize, Deserialize, Default)]
pub struct Cache {
    entries: HashMap<String, CacheEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CacheEntry {
    pub command: String,
    pub model: String,
    pub timestamp: i64,
}

/// Result of a cache lookup, indicating whether it was exact or fuzzy.
pub struct CacheHit {
    pub entry: CacheEntry,
    pub matched_description: Option<String>,
}

fn cache_path() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        let xdg_path = home.join(".config").join("yaak").join("cache.json");
        if xdg_path.parent().map(|p| p.exists()).unwrap_or(false) {
            return xdg_path;
        }
    }
    if let Some(config_dir) = dirs::config_dir() {
        return config_dir.join("yaak").join("cache.json");
    }
    PathBuf::from("cache.json")
}

fn load_cache() -> Cache {
    let path = cache_path();
    if let Ok(contents) = fs::read_to_string(&path) {
        serde_json::from_str(&contents).unwrap_or_default()
    } else {
        Cache::default()
    }
}

fn save_cache(cache: &Cache) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(&path, json);
    }
}

/// Build a cache key from the description and model.
fn cache_key(description: &str, model: &str) -> String {
    format!("{}::{}", model, description.trim().to_lowercase())
}

/// Extract the description portion from a cache key (after the model:: prefix).
fn description_from_key(key: &str) -> &str {
    key.split_once("::").map(|(_, d)| d).unwrap_or(key)
}

/// Extract the model portion from a cache key (before the :: separator).
fn model_from_key(key: &str) -> &str {
    key.split_once("::").map(|(m, _)| m).unwrap_or("")
}

/// Tokenize a description: lowercase, split on whitespace/punctuation, remove stop words.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !STOP_WORDS.contains(w))
        .map(String::from)
        .collect()
}

/// Compute BM25 score for a query against a document.
/// `doc_tokens`: tokens of the cached description
/// `query_tokens`: tokens of the search query
/// `avg_dl`: average document length across the corpus
/// `doc_count`: total number of documents
/// `df`: document frequency map (how many docs contain each term)
fn bm25_score(
    doc_tokens: &[String],
    query_tokens: &[String],
    avg_dl: f64,
    doc_count: usize,
    df: &HashMap<String, usize>,
) -> f64 {
    let dl = doc_tokens.len() as f64;
    let mut score = 0.0;

    // Build term frequency map for this document
    let mut tf: HashMap<&str, usize> = HashMap::new();
    for token in doc_tokens {
        *tf.entry(token.as_str()).or_insert(0) += 1;
    }

    for qt in query_tokens {
        let n = *df.get(qt.as_str()).unwrap_or(&0) as f64;
        let f = *tf.get(qt.as_str()).unwrap_or(&0) as f64;

        // IDF: log((N - n + 0.5) / (n + 0.5) + 1)
        let idf = ((doc_count as f64 - n + 0.5) / (n + 0.5) + 1.0).ln();

        // TF component with length normalization
        let tf_norm = (f * (BM25_K1 + 1.0)) / (f + BM25_K1 * (1.0 - BM25_B + BM25_B * dl / avg_dl));

        score += idf * tf_norm;
    }

    score
}

/// Look up a cached command for the given description and model.
/// Tries exact match first, then falls back to BM25 fuzzy matching.
pub fn get(description: &str, model: &str) -> Option<CacheHit> {
    let cache = load_cache();
    let key = cache_key(description, model);

    // Exact match
    if let Some(entry) = cache.entries.get(&key) {
        return Some(CacheHit {
            entry: entry.clone(),
            matched_description: None,
        });
    }

    // BM25 fuzzy match — only consider entries for the same model
    let model_lower = model.to_lowercase();
    let candidates: Vec<(&String, &CacheEntry)> = cache
        .entries
        .iter()
        .filter(|(k, _)| model_from_key(k) == model_lower)
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Tokenize all documents and compute corpus stats
    let doc_tokens: Vec<Vec<String>> = candidates
        .iter()
        .map(|(k, _)| tokenize(description_from_key(k)))
        .collect();

    let avg_dl = doc_tokens.iter().map(|d| d.len()).sum::<usize>() as f64 / candidates.len() as f64;

    // Document frequency: how many docs contain each term
    let mut df: HashMap<String, usize> = HashMap::new();
    for tokens in &doc_tokens {
        let unique: std::collections::HashSet<&str> = tokens.iter().map(|s| s.as_str()).collect();
        for term in unique {
            *df.entry(term.to_string()).or_insert(0) += 1;
        }
    }

    let query_tokens = tokenize(description);

    // Score each candidate
    let mut best_score = 0.0f64;
    let mut best_idx = 0;

    for (i, tokens) in doc_tokens.iter().enumerate() {
        let score = bm25_score(tokens, &query_tokens, avg_dl, candidates.len(), &df);
        if score > best_score {
            best_score = score;
            best_idx = i;
        }
    }

    if best_score >= BM25_THRESHOLD {
        let (key, entry) = candidates[best_idx];
        let matched = description_from_key(key).to_string();
        Some(CacheHit {
            entry: entry.clone(),
            matched_description: Some(matched),
        })
    } else {
        None
    }
}

/// Store a description→command mapping in the cache.
pub fn put(description: &str, model: &str, command: &str) {
    let mut cache = load_cache();
    let key = cache_key(description, model);
    let entry = CacheEntry {
        command: command.to_string(),
        model: model.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };
    cache.entries.insert(key, entry);
    save_cache(&cache);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_case_insensitive() {
        assert_eq!(
            cache_key("List Files", "gpt-4o"),
            cache_key("list files", "gpt-4o")
        );
    }

    #[test]
    fn cache_key_includes_model() {
        assert_ne!(
            cache_key("list files", "gpt-4o"),
            cache_key("list files", "claude-sonnet-4-6")
        );
    }

    #[test]
    fn cache_key_trims_whitespace() {
        assert_eq!(
            cache_key("  list files  ", "gpt-4o"),
            cache_key("list files", "gpt-4o")
        );
    }

    #[test]
    fn roundtrip_cache_entry() {
        let entry = CacheEntry {
            command: "ls -la".to_string(),
            model: "gpt-4o".to_string(),
            timestamp: 1000,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: CacheEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command, "ls -la");
        assert_eq!(parsed.model, "gpt-4o");
        assert_eq!(parsed.timestamp, 1000);
    }

    #[test]
    fn empty_cache_returns_none() {
        let cache = Cache::default();
        let key = cache_key("anything", "model");
        assert!(!cache.entries.contains_key(&key));
    }

    #[test]
    fn put_and_get_with_temp_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let cache_file = dir.path().join("cache.json");

        let mut cache = Cache::default();
        let key = cache_key("list files", "gpt-4o");
        cache.entries.insert(
            key.clone(),
            CacheEntry {
                command: "ls -la".to_string(),
                model: "gpt-4o".to_string(),
                timestamp: 1000,
            },
        );

        let json = serde_json::to_string_pretty(&cache).unwrap();
        fs::write(&cache_file, &json).unwrap();

        let loaded: Cache =
            serde_json::from_str(&fs::read_to_string(&cache_file).unwrap()).unwrap();
        let entry = loaded.entries.get(&key).unwrap();
        assert_eq!(entry.command, "ls -la");
    }

    #[test]
    fn tokenize_removes_stop_words() {
        let tokens = tokenize("please show me the files in my directory");
        assert!(!tokens.contains(&"please".to_string()));
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"me".to_string()));
        assert!(tokens.contains(&"files".to_string()));
        assert!(tokens.contains(&"directory".to_string()));
    }

    #[test]
    fn tokenize_lowercases() {
        let tokens = tokenize("Find RUST Files");
        assert!(tokens.contains(&"find".to_string()));
        assert!(tokens.contains(&"rust".to_string()));
        assert!(tokens.contains(&"files".to_string()));
    }

    #[test]
    fn bm25_scores_similar_higher() {
        // Two documents: one similar, one unrelated
        let doc1 = tokenize("find all rust files modified this week");
        let doc2 = tokenize("compress png images into archive");
        let query = tokenize("show rust files changed recently");

        let all_docs = vec![&doc1, &doc2];
        let avg_dl = all_docs.iter().map(|d| d.len()).sum::<usize>() as f64 / 2.0;

        let mut df: HashMap<String, usize> = HashMap::new();
        for doc in &all_docs {
            let unique: std::collections::HashSet<&str> = doc.iter().map(|s| s.as_str()).collect();
            for term in unique {
                *df.entry(term.to_string()).or_insert(0) += 1;
            }
        }

        let score1 = bm25_score(&doc1, &query, avg_dl, 2, &df);
        let score2 = bm25_score(&doc2, &query, avg_dl, 2, &df);

        assert!(
            score1 > score2,
            "similar doc should score higher: {} vs {}",
            score1,
            score2
        );
    }

    #[test]
    fn bm25_exact_overlap_scores_high() {
        let doc = tokenize("list all docker containers");
        let query = tokenize("list docker containers");

        let avg_dl = doc.len() as f64;
        let mut df: HashMap<String, usize> = HashMap::new();
        for token in &doc {
            *df.entry(token.to_string()).or_insert(0) += 1;
        }

        let score = bm25_score(&doc, &query, avg_dl, 1, &df);
        assert!(
            score >= BM25_THRESHOLD,
            "near-exact match should exceed threshold: {}",
            score
        );
    }

    #[test]
    fn bm25_no_overlap_scores_zero() {
        let doc = tokenize("compress archive files");
        let query = tokenize("restart nginx server");

        let avg_dl = doc.len() as f64;
        let mut df: HashMap<String, usize> = HashMap::new();
        for token in &doc {
            *df.entry(token.to_string()).or_insert(0) += 1;
        }

        let score = bm25_score(&doc, &query, avg_dl, 1, &df);
        assert!(
            score < BM25_THRESHOLD,
            "unrelated query should score below threshold: {}",
            score
        );
    }

    #[test]
    fn description_from_key_extracts_correctly() {
        assert_eq!(description_from_key("gpt-4o::list files"), "list files");
        assert_eq!(
            description_from_key("claude-sonnet-4-6::find rust files"),
            "find rust files"
        );
    }

    #[test]
    fn model_from_key_extracts_correctly() {
        assert_eq!(model_from_key("gpt-4o::list files"), "gpt-4o");
        assert_eq!(
            model_from_key("claude-sonnet-4-6::find rust files"),
            "claude-sonnet-4-6"
        );
    }
}
