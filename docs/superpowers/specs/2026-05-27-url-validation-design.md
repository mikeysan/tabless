# URL Validation Subsystem Design

## Overview

The URL validation subsystem is the first line of defense for the application. It receives untrusted URL strings from OS protocol handlers, UI paste operations, and future import flows, and transforms them into a strongly-typed, canonicalised, validated representation.

## Goals

- Parse raw URL strings safely using a battle-tested parser.
- Enforce a strict scheme allowlist (`http`, `https`).
- Reject malformed, obfuscated, or potentially malicious input.
- Produce a canonical form for deduplication and stable storage.
- Preserve the original input for user reference.
- Return fine-grained, typed errors for every failure mode.
- Contain zero panics and zero unwraps in production paths.

## Non-Goals

- Tracking parameter stripping (deferred to a later iteration).
- Embedded rendering or preview of URL content.
- Support for non-HTTP schemes in the MVP.

## Approach

**Approach 2: `url` crate + strict wrapper.**

The `url` crate is the de-facto standard for Rust URL parsing. It handles edge cases such as percent-encoding, punycode, IDN, and port validation correctly. Rather than reimplementing parsing, we wrap the crate and enforce our own constraints at the boundary.

A `ValidatedUrl` struct wraps the parsed result. Once constructed, it is a proof-carrying type: downstream code can trust that the URL has passed all validation rules.

## Public API

```rust
pub struct ValidatedUrl {
    original: String,
    canonical: Url,
}

impl ValidatedUrl {
    pub fn parse(input: &str) -> Result<Self, UrlValidationError>;
    pub fn original(&self) -> &str;
    pub fn canonical(&self) -> &str;
    pub fn scheme(&self) -> &str;
    pub fn host(&self) -> &str;
}
```

## Error Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum UrlValidationError {
    EmptyInput,
    InvalidScheme { found: String },
    MalformedUrl { reason: String },
    EmptyHost,
    InvalidPort { port: u16 },
}
```

## Module Structure

```
src/
  url/
    mod.rs          -- public API: ValidatedUrl, UrlValidationError
    parser.rs       -- internal: wraps url::Url, enforces scheme allowlist
    normalizer.rs   -- internal: canonicalisation rules
    error.rs        -- internal: error enum definitions
```

### parser.rs

- Uses `url::Url::parse` for initial parsing.
- Checks scheme against allowlist: `http`, `https`.
- Rejects empty or missing host.
- Rejects invalid or non-standard ports.
- Maps `url` crate errors into our `UrlValidationError` variants.

### normalizer.rs

- Lowercases hostnames.
- Preserves the original URL unchanged.
- Produces a deterministic canonical form for storage and deduplication.

### error.rs

- Defines `UrlValidationError`.
- Implements `Display` for user-facing messages.
- Implements `std::error::Error` for compatibility.

## Data Flow

```
Raw URL string
        |
        v
ValidatedUrl::parse(input)
        |
        +-- url::Url::parse(input)
        +-- scheme allowlist check
        +-- host validation
        +-- normalizer: lowercase hostname
        |
        v
ValidatedUrl { original, canonical }
```

## Normalisation Rules (MVP)

1. Lowercase hostnames.
2. Preserve original URL separately.
3. No tracking parameter stripping in MVP.

## Security Considerations

- All input is treated as hostile.
- Shell injection is impossible: the module never invokes a shell.
- Scheme allowlist prevents `javascript:`, `data:`, `file:`, and other dangerous schemes.
- Unicode spoofing is mitigated by the `url` crate's IDN and punycode handling.

## Testing Strategy

- Unit tests for every `UrlValidationError` variant.
- Unit tests for normalisation (host lowercasing, canonical stability).
- Edge-case tests: unicode paths, percent-encoding, punycode, valid but unusual ports.
- Property-based tests may be added later via `proptest`.

## Success Criteria

- `ValidatedUrl::parse` accepts well-formed `http` and `https` URLs.
- It rejects every other scheme with `InvalidScheme`.
- It rejects malformed input with `MalformedUrl`.
- It rejects empty hosts with `EmptyHost`.
- Canonical hostnames are always lowercase.
- Original URLs are preserved exactly.
- No panics or unwraps in production paths.
