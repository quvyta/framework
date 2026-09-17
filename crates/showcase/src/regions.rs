//! Extracting the code a page shows from its own source: the lines between
//! `// region: name` and `// endregion`.
//!
//! rustfmt moves an `// endregion` that follows the last arm of a `match` onto the arm's closing
//! line (`} // endregion`), so a marker that ends a line of code closes the region too.

/// One named piece of source code, with common indentation removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub name: String,
    pub code: String,
}

/// The marker that closes a region.
const END: &str = "// endregion";

/// Every region in `source`, in order.
#[must_use]
pub fn extract(source: &str) -> Vec<Region> {
    let mut regions = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("// region:") {
            current = Some((name.trim().to_owned(), Vec::new()));
        } else if let Some(code) = line.trim_end().strip_suffix(END) {
            if let Some((name, mut lines)) = current.take() {
                if !code.trim().is_empty() {
                    lines.push(code);
                }
                regions.push(Region { name, code: dedent(&lines) });
            }
        } else if let Some((_, lines)) = &mut current {
            lines.push(line);
        }
    }
    regions
}

fn dedent(lines: &[&str]) -> String {
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    lines.iter().map(|line| line.get(indent..).unwrap_or("").trim_end()).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_dedents_regions() {
        let source = "fn a() {\n    // region: basic\n    let x = 1;\n        nested();\n    // endregion\n}\n";
        let regions = extract(source);
        assert_eq!(regions, vec![Region { name: "basic".into(), code: "let x = 1;\n    nested();".into() }]);
    }

    #[test]
    fn a_marker_after_code_closes_the_region_and_keeps_the_code() {
        let source = "match m {\n    // region: back\n    Msg::Back => {\n        back();\n    } // endregion\n}\n";
        let regions = extract(source);
        assert_eq!(regions, vec![Region { name: "back".into(), code: "Msg::Back => {\n    back();\n}".into() }]);
    }
}
