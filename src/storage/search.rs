use sublime_fuzzy::{FuzzySearch, Scoring};

use super::error::StorageError;
use super::url_repo::UrlRepository;

pub struct FuzzySearchIndex {
    candidates: Vec<String>,
    scoring: Scoring,
}

impl FuzzySearchIndex {
    pub fn new() -> Self {
        FuzzySearchIndex {
            candidates: Vec::new(),
            scoring: Scoring::default(),
        }
    }

    pub fn rebuild(&mut self, repo: &UrlRepository) -> Result<(), StorageError> {
        self.candidates.clear();
        let urls = repo.list_inbox()?;
        for url in urls {
            self.candidates.push(url.canonical_url.clone());
            if let Some(title) = url.title {
                self.candidates.push(title);
            }
        }
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .candidates
            .iter()
            .filter_map(|candidate| {
                FuzzySearch::new(query, candidate)
                    .score_with(&self.scoring)
                    .best_match()
                    .map(|m| SearchResult {
                        text: candidate.clone(),
                        score: m.score() as i64,
                    })
            })
            .collect();

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(20);
        results
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub text: String,
    pub score: i64,
}
