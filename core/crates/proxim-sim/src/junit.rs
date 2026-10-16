//! JUnit XML for CI: one `testcase` per scenario, a `failure` element listing the oracle's
//! first failures. No timestamps or durations, so the file is as deterministic as the report.

use crate::runner::ScenarioResult;
use alloc::string::String;
use serde_json::Value;

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' => o.push(' '),
            c => o.push(c),
        }
    }
    o
}

fn failure_lines(r: &ScenarioResult) -> String {
    let mut lines = String::new();
    let mut push = |v: &Value| {
        if let Some(s) = v.as_str() {
            lines.push_str(s);
            lines.push('\n');
        }
    };
    if let Some(sps) = r.summary.get("signposts").and_then(Value::as_array) {
        for sp in sps {
            if sp.get("pass") == Some(&Value::Bool(false))
                && let Some(f) = sp.get("failures").and_then(Value::as_array)
            {
                f.iter().for_each(&mut push);
            }
        }
    }
    if let Some(g) = r.summary.get("global_failures").and_then(Value::as_array) {
        g.iter().for_each(&mut push);
    }
    lines
}

/// Render JUnit XML for the results.
#[must_use]
pub fn junit_xml(results: &[ScenarioResult]) -> String {
    let failed = results.iter().filter(|r| !r.pass).count();
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str(&alloc::format!(
        "<testsuites name=\"proxim-sim\" tests=\"{}\" failures=\"{failed}\">\n<testsuite name=\"scenarios\" tests=\"{}\" failures=\"{failed}\" errors=\"0\" skipped=\"0\">\n",
        results.len(),
        results.len()
    ));
    for r in results {
        s.push_str(&alloc::format!(
            "  <testcase classname=\"proxim-sim.scenarios\" name=\"{}\">",
            esc(&r.id)
        ));
        if r.pass {
            s.push_str("</testcase>\n");
        } else {
            s.push_str(&alloc::format!(
                "\n    <failure message=\"{} failed over {} run(s)\">{}</failure>\n  </testcase>\n",
                esc(&r.id),
                r.runs,
                esc(&failure_lines(r))
            ));
        }
    }
    s.push_str("</testsuite>\n</testsuites>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::esc;

    #[test]
    fn escapes_xml() {
        assert_eq!(esc("a<b & \"c\"\u{1}"), "a&lt;b &amp; &quot;c&quot; ");
    }
}
