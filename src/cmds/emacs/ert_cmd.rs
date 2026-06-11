//! Filters Emacs ERT batch output (`ert-run-tests-batch-and-exit`).
//!
//! A green run collapses to its one-line summary; a run with failures drops the
//! per-test `passed`/`skipped` progress spam but keeps every failure block, the
//! `FAILED` lines, and the final summary in full. ERT puts the verdict at the
//! bottom, so a head-cap (the generic `just.toml` fallback) throws away exactly
//! what you need — this keeps it.

use crate::core::utils::strip_ansi;
use regex::Regex;
use std::sync::LazyLock;

/// Compact ERT batch output. Returns the run summary alone when everything
/// passed, otherwise the failures verbatim minus the passed/skipped noise.
pub fn filter_ert_output(output: &str) -> String {
    // Per-test progress lines dropped on a failing run (FAILED is kept).
    static PASS_LINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(passed|skipped)\s+\d+/\d+\s").unwrap());
    // "Running 243 tests (...)" header.
    static HEADER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^Running \d+ tests\b").unwrap());
    // "Ran 243 tests, 243 results as expected[, 1 unexpected] (...)".
    static FOOTER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^Ran \d+ tests?, .*results as expected").unwrap());

    let clean = strip_ansi(output);
    let lines: Vec<&str> = clean.lines().collect();
    let footer = lines.iter().rev().find(|l| FOOTER.is_match(l)).copied();

    match footer {
        // Green run (no unexpected results): just the one-line ack.
        Some(f) if !f.contains("unexpected") => f.trim().to_string(),

        // Failing run, or no footer at all (crash / early abort): keep failures
        // in full; drop only the passed/skipped progress spam and the header.
        _ => lines
            .iter()
            .filter(|l| !PASS_LINE.is_match(l) && !HEADER.is_match(l))
            .copied()
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_run_collapses_to_summary() {
        let input = "\
Running 3 tests (2026-06-11 10:00:00+0000, selector t)
   passed  1/3  cr-test--a (0.001 sec)
   passed  2/3  cr-test--b (0.002 sec)
   passed  3/3  cr-test--c (0.000 sec)

Ran 3 tests, 3 results as expected (2026-06-11 10:00:01+0000)";
        assert_eq!(
            filter_ert_output(input),
            "Ran 3 tests, 3 results as expected (2026-06-11 10:00:01+0000)"
        );
    }

    #[test]
    fn failing_run_keeps_failures_drops_passed() {
        let input = "\
Running 3 tests (2026-06-11 10:00:00+0000, selector t)
   passed  1/3  cr-test--a (0.001 sec)
Test cr-test--b condition:
    (ert-test-failed (\"boom\"))
   FAILED  2/3  cr-test--b (0.002 sec)
   passed  3/3  cr-test--c (0.000 sec)

Ran 3 tests, 2 results as expected, 1 unexpected (2026-06-11 10:00:01+0000)

1 unexpected results:
   FAILED  cr-test--b";
        let out = filter_ert_output(input);
        assert!(out.contains("FAILED  2/3  cr-test--b"), "keeps the FAILED line");
        assert!(out.contains("condition:"), "keeps the failure detail block");
        assert!(out.contains("1 unexpected"), "keeps the summary");
        assert!(out.contains("1 unexpected results:"), "keeps the trailer");
        assert!(!out.contains("passed  1/3"), "drops passed spam");
        assert!(!out.contains("Running 3 tests"), "drops the header");
    }

    #[test]
    fn no_footer_passes_through_body() {
        // Emacs aborted mid-run (e.g. a load error) — keep whatever we have.
        let input = "Test cr-test--a backtrace:\n  (void-function foo)\n";
        let out = filter_ert_output(input);
        assert!(out.contains("void-function foo"));
    }
}
