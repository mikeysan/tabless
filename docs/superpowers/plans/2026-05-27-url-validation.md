# URL Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a strict URL validation subsystem that parses raw strings into a strongly-typed `ValidatedUrl`, enforcing scheme allowlists, host validation, and canonicalisation.

**Architecture:** Wrap the `url` crate in a custom `ValidatedUrl` struct. Split concerns across focused modules: errors, parsing, normalisation. TDD throughout — write failing tests first, then minimal implementation.

**Tech Stack:** Rust, `url` crate, standard library only.

---

## File Structure

```
src/
  main.rs          -- application entry point (minimal stub)
  lib.rs           -- library root, re-exports
  url/
    mod.rs         -- public API: re-exports ValidatedUrl and UrlValidationError
    error.rs       -- UrlValidationError enum with Display + std::error::Error
    parser.rs      -- ValidatedUrl struct and parse logic
    normalizer.rs  -- canonicalisation: lowercase hostnames

tests/
  url_integration_tests.rs  -- integration tests for public API boundary
```

---

### Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "tabless"
version = "0.1.0"
edition = "2024"

[dependencies]
url = "2.5"

[dev-dependencies]
```

- [ ] **Step 2: Create src/main.rs**

```rust
fn main() {
    println!("tabless - URL capture and launch utility");
}
```

- [ ] **Step 3: Create src/lib.rs**

```rust
pub mod url;
```

- [ ] **Step 4: Verify project compiles**

Run: `cargo check`
Expected: Compiles successfully with no warnings.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs src/lib.rs
git commit -m "chore: scaffold Rust project"
```

---

### Task 2: Define Error Types

**Files:**
- Create: `src/url/error.rs`
- Create: `src/url/mod.rs`

- [ ] **Step 1: Write failing test for error type existence**

Create `tests/url_integration_tests.rs`:

```rust
use tabless::url::UrlValidationError;

#[test]
fn error_variants_exist() {
    let _e = UrlValidationError::EmptyInput;
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test url_integration_tests`
Expected: FAIL — `UrlValidationError` not found.

- [ ] **Step 3: Implement UrlValidationError**

Create `src/url/error.rs`:

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum UrlValidationError {
    EmptyInput,
    InvalidScheme { found: String },
    MalformedUrl { reason: String },
    EmptyHost,
    InvalidPort { port: u16 },
}

impl fmt::Display for UrlValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlValidationError::EmptyInput => write!(f, "URL input is empty"),
            UrlValidationError::InvalidScheme { found } => {
                write!(f, "invalid URL scheme: {}", found)
            }
            UrlValidationError::MalformedUrl { reason } => {
                write!(f, "malformed URL: {}", reason)
            }
            UrlValidationError::EmptyHost => write!(f, "URL has no host"),
            UrlValidationError::InvalidPort { port } => {
                write!(f, "invalid port number: {}", port)
            }
        }
    }
}

impl std::error::Error for UrlValidationError {}
```

- [ ] **Step 4: Wire up url module**

Create `src/url/mod.rs`:

```rust
pub mod error;

pub use error::UrlValidationError;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test url_integration_tests`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/url/error.rs src/url/mod.rs tests/url_integration_tests.rs
git commit -m "feat: define UrlValidationError enum"
```

---

### Task 3: Implement Normalizer

**Files:**
- Create: `src/url/normalizer.rs`
- Modify: `src/url/mod.rs`

- [ ] **Step 1: Write failing test for normalizer**

Add to `tests/url_integration_tests.rs`:

```rust
use tabless::url::normalizer::normalize;

#[test]
fn normalizer_lowercases_hostname() {
    let input = url::Url::parse("https://EXAMPLE.COM/path").unwrap();
    let normalized = normalize(&input);
    assert_eq!(normalized.host_str(), Some("example.com"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test url_integration_tests normalizer_lowercases_hostname`
Expected: FAIL — `normalizer` module not found.

- [ ] **Step 3: Implement normalizer**

Create `src/url/normalizer.rs`:

```rust
use url::Url;

pub fn normalize(url: &Url) -> Url {
    let mut normalized = url.clone();
    if let Some(host) = normalized.host_str() {
        let lower = host.to_lowercase();
        if host != lower {
            let _ = normalized.set_host(Some(&lower));
        }
    }
    normalized
}
```

- [ ] **Step 4: Wire up normalizer module**

Modify `src/url/mod.rs`:

```rust
pub mod error;
pub mod normalizer;

pub use error::UrlValidationError;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test url_integration_tests normalizer_lowercases_hostname`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/url/normalizer.rs src/url/mod.rs tests/url_integration_tests.rs
git commit -m "feat: add URL normalizer with hostname lowercasing"
```

---

### Task 4: Implement Parser and ValidatedUrl

**Files:**
- Create: `src/url/parser.rs`
- Modify: `src/url/mod.rs`
- Modify: `tests/url_integration_tests.rs`

- [ ] **Step 1: Write failing integration tests for ValidatedUrl**

Replace contents of `tests/url_integration_tests.rs`:

```rust
use tabless::url::{ValidatedUrl, UrlValidationError};

#[test]
fn parse_valid_http_url() {
    let result = ValidatedUrl::parse("http://example.com");
    assert!(result.is_ok());
    let validated = result.unwrap();
    assert_eq!(validated.original(), "http://example.com");
    assert_eq!(validated.canonical(), "http://example.com/");
    assert_eq!(validated.scheme(), "http");
    assert_eq!(validated.host(), "example.com");
}

#[test]
fn parse_valid_https_url() {
    let result = ValidatedUrl::parse("https://example.com");
    assert!(result.is_ok());
}

#[test]
fn parse_rejects_empty_input() {
    let result = ValidatedUrl::parse("");
    assert!(matches!(result, Err(UrlValidationError::EmptyInput)));
}

#[test]
fn parse_rejects_invalid_scheme() {
    let result = ValidatedUrl::parse("javascript:alert(1)");
    assert!(matches!(
        result,
        Err(UrlValidationError::InvalidScheme { found })
        if found == "javascript"
    ));
}

#[test]
fn parse_rejects_file_scheme() {
    let result = ValidatedUrl::parse("file:///etc/passwd");
    assert!(matches!(
        result,
        Err(UrlValidationError::InvalidScheme { found })
        if found == "file"
    ));
}

#[test]
fn parse_rejects_malformed_url() {
    let result = ValidatedUrl::parse("not a url");
    assert!(matches!(result, Err(UrlValidationError::MalformedUrl { .. })));
}

#[test]
fn parse_lowercases_hostname() {
    let validated = ValidatedUrl::parse("https://EXAMPLE.COM/path").unwrap();
    assert_eq!(validated.host(), "example.com");
    assert_eq!(validated.canonical(), "https://example.com/path");
}

#[test]
fn parse_preserves_original_url() {
    let original = "https://EXAMPLE.COM/path?query=1";
    let validated = ValidatedUrl::parse(original).unwrap();
    assert_eq!(validated.original(), original);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test url_integration_tests`
Expected: FAIL — `ValidatedUrl` not found.

- [ ] **Step 3: Implement parser.rs**

Create `src/url/parser.rs`:

```rust
use url::Url;

use super::error::UrlValidationError;
use super::normalizer::normalize;

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedUrl {
    original: String,
    canonical: Url,
}

impl ValidatedUrl {
    pub fn parse(input: &str) -> Result<Self, UrlValidationError> {
        if input.is_empty() {
            return Err(UrlValidationError::EmptyInput);
        }

        let parsed = Url::parse(input).map_err(|e| UrlValidationError::MalformedUrl {
            reason: e.to_string(),
        })?;

        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(UrlValidationError::InvalidScheme {
                found: scheme.to_string(),
            });
        }

        if parsed.host_str().is_none() {
            return Err(UrlValidationError::EmptyHost);
        }

        let canonical = normalize(&parsed);

        Ok(ValidatedUrl {
            original: input.to_string(),
            canonical,
        })
    }

    pub fn original(&self) -> &str {
        &self.original
    }

    pub fn canonical(&self) -> &str {
        self.canonical.as_str()
    }

    pub fn scheme(&self) -> &str {
        self.canonical.scheme()
    }

    pub fn host(&self) -> &str {
        self.canonical.host_str().unwrap_or("")
    }
}
```

- [ ] **Step 4: Wire up parser module**

Modify `src/url/mod.rs`:

```rust
pub mod error;
pub mod normalizer;
pub mod parser;

pub use error::UrlValidationError;
pub use parser::ValidatedUrl;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test url_integration_tests`
Expected: All 8 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/url/parser.rs src/url/mod.rs tests/url_integration_tests.rs
git commit -m "feat: implement ValidatedUrl parser with scheme allowlisting"
```

---

### Task 5: Add Unit Tests for Edge Cases

**Files:**
- Modify: `src/url/parser.rs`
- Modify: `src/url/normalizer.rs`
- Modify: `src/url/error.rs`

- [ ] **Step 1: Add unit tests to parser.rs**

Append to `src/url/parser.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_https_with_path_and_query() {
        let v = ValidatedUrl::parse("https://example.com/path?foo=bar").unwrap();
        assert_eq!(v.canonical(), "https://example.com/path?foo=bar");
    }

    #[test]
    fn parse_rejects_data_scheme() {
        let result = ValidatedUrl::parse("data:text/html,hello");
        assert!(matches!(
            result,
            Err(UrlValidationError::InvalidScheme { found })
            if found == "data"
        ));
    }

    #[test]
    fn parse_rejects_about_scheme() {
        let result = ValidatedUrl::parse("about:blank");
        assert!(matches!(
            result,
            Err(UrlValidationError::InvalidScheme { found })
            if found == "about"
        ));
    }

    #[test]
    fn parse_accepts_punycode_hostname() {
        let v = ValidatedUrl::parse("https://xn--example-9ua.com").unwrap();
        assert_eq!(v.host(), "xn--example-9ua.com");
    }

    #[test]
    fn parse_preserves_percent_encoding() {
        let v = ValidatedUrl::parse("https://example.com/hello%20world").unwrap();
        assert_eq!(v.canonical(), "https://example.com/hello%20world");
    }

    #[test]
    fn parse_rejects_missing_host() {
        let result = ValidatedUrl::parse("http:///path");
        assert!(matches!(result, Err(UrlValidationError::EmptyHost)));
    }

    #[test]
    fn parse_accepts_custom_port() {
        let v = ValidatedUrl::parse("https://example.com:8080").unwrap();
        assert_eq!(v.canonical(), "https://example.com:8080/");
    }
}
```

- [ ] **Step 2: Add unit tests to normalizer.rs**

Append to `src/url/normalizer.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_already_lowercase_noop() {
        let url = Url::parse("https://example.com").unwrap();
        let normalized = normalize(&url);
        assert_eq!(normalized.host_str(), Some("example.com"));
    }

    #[test]
    fn normalize_uppercase_host() {
        let url = Url::parse("https://EXAMPLE.COM").unwrap();
        let normalized = normalize(&url);
        assert_eq!(normalized.host_str(), Some("example.com"));
    }

    #[test]
    fn normalize_mixed_case_host() {
        let url = Url::parse("https://ExAmPlE.CoM").unwrap();
        let normalized = normalize(&url);
        assert_eq!(normalized.host_str(), Some("example.com"));
    }

    #[test]
    fn normalize_preserves_path_and_query() {
        let url = Url::parse("https://EXAMPLE.COM/PATH?Q=1").unwrap();
        let normalized = normalize(&url);
        assert_eq!(normalized.as_str(), "https://example.com/PATH?Q=1");
    }
}
```

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: All unit and integration tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/url/parser.rs src/url/normalizer.rs
git commit -m "test: add edge-case unit tests for parser and normalizer"
```

---

### Task 6: Final Verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests PASS.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Run formatter**

Run: `cargo fmt`
Expected: Formats cleanly, no changes needed (or changes applied).

- [ ] **Step 4: Build release**

Run: `cargo build --release`
Expected: Compiles successfully.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: final verification and formatting"
```

---

## Self-Review Checklist

| Spec Requirement | Task | Status |
|---|---|---|
| Parse raw URL strings safely | Task 4 | Covered |
| Scheme allowlist (`http`, `https`) | Task 4 | Covered |
| Reject malformed input | Task 4 | Covered |
| Canonical form for deduplication | Task 3 | Covered |
| Preserve original input | Task 4 | Covered |
| Fine-grained typed errors | Task 2, Task 4 | Covered |
| Zero panics/unwraps in production | All tasks | Covered |
| No tracking parameter stripping (deferred) | N/A (non-goal) | Correctly excluded |

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-27-url-validation.md`. Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach would you prefer?
