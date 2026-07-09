//! Output classifier for wrapped commands (e.g. `just` recipes, docker-exec)
//! where the hook can't see the inner tool from the command line.
//!
//! It sniffs the captured output for a tool's signature and applies that
//! tool's specialized filter. Returns `None` when nothing matches, so the
//! caller falls back to its generic filter (e.g. the `just.toml` line cap).

use crate::cmds;

/// Inspect captured command output; if it matches a known tool's output shape,
/// return the specialized-filtered result. `None` ⇒ no signature matched.
///
/// Order matters: most specific signatures are checked first.
pub fn classify_and_filter(output: &str) -> Option<String> {
    // TypeScript compiler — "error TS2322" etc.
    if output.contains("error TS") {
        let filtered = cmds::js::tsc_cmd::filter_tsc_output(output);
        // The formatter returns this success-like message when none of its
        // canonical diagnostics parsed. Do not let a loose signature hide a
        // failing wrapped command; fall back to the generic `just` filter.
        if filtered != "TypeScript compilation completed" {
            return Some(filtered);
        }
    }

    // Emacs ERT batch — "Ran N tests, M results as expected" is ERT-unique.
    if output.contains("results as expected") {
        return Some(cmds::emacs::ert_cmd::filter_ert_output(output));
    }

    // Vitest — run banner / summary line.
    if output.contains("Test Files ") || output.contains("RUN  v") {
        return Some(vitest_filter(output));
    }

    // Pytest — session banner / summary phrasing.
    if output.contains("test session starts")
        || output.contains(" passed in ")
        || output.contains(" failed in ")
        || output.contains("short test summary")
    {
        return Some(cmds::python::pytest_cmd::filter_pytest_output(output));
    }

    // mypy — "...source file(s)" appears in both success and failure summaries.
    if (output.contains("source file") && output.contains(": error:"))
        || output.contains("Success: no issues found in")
    {
        return Some(cmds::python::mypy_cmd::filter_mypy_output(output));
    }

    // eslint / biome stylish (non-JSON) lint output.
    if output.contains(" problems (") || output.contains("lint/") {
        return Some(cmds::js::lint_cmd::filter_generic_lint(output));
    }

    None
}

fn vitest_filter(input: &str) -> String {
    use crate::cmds::js::vitest_cmd::VitestParser;
    use crate::parser::{FormatMode, OutputParser, TokenFormatter};
    match VitestParser::parse(input) {
        crate::parser::ParseResult::Full(d) => d.format(FormatMode::Compact),
        crate::parser::ParseResult::Degraded(d, _) => d.format(FormatMode::Compact),
        crate::parser::ParseResult::Passthrough(raw) => raw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unparseable_typescript_error_falls_back() {
        // Carries the `error TS` signature but matches none of the three
        // diagnostic forms tsc_cmd knows: the code is not followed by a colon,
        // so neither the `file(l,c):` nor the `file:l:c -` nor the global
        // `error TSxxxx:` pattern applies.
        let output = "src/main.ts:4:1 - error TS2322 Type string is not assignable to number.";

        assert_eq!(classify_and_filter(output), None);
    }

    #[test]
    fn canonical_typescript_error_uses_specialized_filter() {
        let output = "src/main.ts(4,1): error TS2322: Type string is not assignable to number.";

        let filtered = classify_and_filter(output).expect("canonical tsc output should classify");
        assert!(filtered.contains("TS2322"));
        assert!(filtered.contains("main.ts"));
    }
}
