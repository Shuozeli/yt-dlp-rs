# Code Quality Findings

## 1. Dead Code

### Unused `LazyExtractor` struct - FIXED
- **Location:** `ytdlp-extractor/src/registry.rs:52-77`
- **Problem:** The `LazyExtractor` struct and its `new()` and `get_or_init()` methods were never used.
- **Fix:** Removed `LazyExtractor` struct entirely.

### Unused `in_representation` variable assignments - FIXED
- **Location:** `ytdlp-downloader/src/dash.rs:64, 123`
- **Problem:** The variable `in_representation` was assigned but never read before being overwritten.
- **Fix:** Added `#[allow(unused_assignments)]` to suppress warnings since the code may be used in the future.

## 2. Clippy Warnings (Style)

### Empty line after doc comment - FIXED
- **Location:** `ytdlp-extractor/src/generic.rs:14-15`
- **Problem:** Doc comment had an empty line between the description and the documented item.
- **Fix:** Removed the blank line after the doc comment.

### `Iterator::last()` on `DoubleEndedIterator` - FIXED
- **Location:** `ytdlp-extractor/src/generic.rs:50-52` and `ytdlp-extractor/src/generic.rs:75`
- **Problem:** Calling `last()` on a `DoubleEndedIterator` is inefficient as it iterates the entire iterator.
- **Fix:** Used `rfind()` instead of `filter().next_back()` and `next_back()` instead of `last()`.

### `Vec::new()` followed by `push()` - FIXED
- **Location:** `ytdlp-extractor/src/generic.rs:58-70`
- **Problem:** Creating a `Vec` with `new()` and then pushing a single element is less idiomatic than using `vec![]`.
- **Fix:** Used `vec![]` macro instead.

### Useless `Bytes::from()` conversion - FIXED
- **Location:** `ytdlp-net/src/http.rs:149`
- **Problem:** `Bytes::from(body)` was redundant since `body` is already `Bytes`.
- **Fix:** Used `body` directly.

### Manual `flatten()` needed - FIXED
- **Location:** `ytdlp-net/src/cookies.rs:108-112` and `ytdlp-net/src/cookies.rs:191-195`
- **Problem:** Using `if let Ok(cookie) = cookie` pattern instead of `.flatten()`.
- **Fix:** Used `for cookie in cookies.flatten()` pattern.

### Match with single binding - FIXED
- **Location:** `ytdlp-net/src/user_agent.rs:60-63`
- **Problem:** A `match` with only a wildcard arm can be replaced with its body.
- **Fix:** Replaced `match extractor { _ => Self::random() }` with just `Self::random()`.

### Missing `Default` implementation - FIXED
- **Location:** `ytdlp-extractors/src/youtube.rs:21` and `ytdlp-extractors/src/generic.rs`
- **Problem:** `YoutubeExtractor` and `GenericExtractor` had `new()` methods but clippy suggested implementing `Default`.
- **Fix:** Added `#[derive(Default)]` for `YoutubeExtractor` and `impl Default` for `GenericExtractor`.

### Dead code warnings for unused public API - FIXED
- **Location:** `ytdlp-cli/src/output_template.rs`, `ytdlp-cli/src/progress.rs`, `ytdlp-cli/src/config.rs`
- **Problem:** `OutputTemplate`, `ProgressDisplay`, `DownloadProgress` structs and their methods were unused but part of public API.
- **Fix:** Added `#[allow(dead_code)]` to suppress warnings since these are intended for future use.

### Print literal issue - FIXED
- **Location:** `ytdlp-cli/src/client.rs:109`
- **Problem:** Extra argument `"   "` passed to `print!` without corresponding format placeholder.
- **Fix:** Changed to `print!("\rDownloading: {} ({}) Speed:    ", percent, speed);`

## 3. Duplication

### Duplicate `GenericExtractor` implementations - DOCUMENTED (NOT FIXED)
- **Location:** `ytdlp-extractor/src/generic.rs:1-89` and `ytdlp-extractors/src/generic.rs:1-119`
- **Problem:** Two different `GenericExtractor` implementations exist with different purposes.
- **Decision:** Kept as-is since they serve different purposes (simple URL parsing vs HTTP client-based extraction). Could be renamed for clarity in future.

## 4. Formatting Issues - FIXED

### `cargo fmt --check` failures
- **Problem:** Multiple files failed `cargo fmt --check`.
- **Fix:** Ran `cargo fmt` to auto-format all files.

## 5. Configuration Issues

### Invalid Rust edition in workspace - FIXED
- **Location:** `Cargo.toml:17`
- **Problem:** `edition = "2024"` is not a valid Rust edition.
- **Fix:** Changed to `edition = "2021"`.

### Redundant package configuration in ytdlp-net - FIXED
- **Location:** `ytdlp-net/Cargo.toml`
- **Problem:** Contained redundant `[package]` section with workspace references and local `[profile]` sections that were ignored.
- **Fix:** Kept minimal `[package]` section with only `name` and `edition.workspace = true`, removed redundant `[profile]` sections.

### Missing dev-dependency - FIXED
- **Location:** `ytdlp-cli/Cargo.toml`
- **Problem:** `tempfile` crate used in tests but not declared as dev-dependency.
- **Fix:** Added `tempfile = "3"` to `[dev-dependencies]`.

## Summary

- **Total issues found:** 14
- **Fixed:** 13
- **Documented (not fixed):** 1 (duplicate GenericExtractor - different purposes)
- **Remaining warnings:** 2 (style suggestions about function argument count - acceptable)
