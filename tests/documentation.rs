use rstest::rstest;

fn without_indentation(text: &str) -> String {
    text.lines().map(str::trim).collect::<Vec<_>>().join("\n")
}

#[rstest]
#[case::dependencies("dependencies", include_str!("../examples/common.rs"))]
#[case::consumer("consumer", include_str!("../examples/common.rs"))]
#[case::shared_stubs("shared-stubs", include_str!("../examples/common.rs"))]
#[case::individual_override("override", include_str!("../examples/common.rs"))]
#[case::custom_types("custom-types", include_str!("../examples/types.rs"))]
#[case::worker_generics("worker-generics", include_str!("../examples/workers.rs"))]
fn readme_snippets_match_the_runnable_example(
    #[case] name: &str,
    #[case] source: &str,
    #[values("\n", "\r\n")] line_ending: &str,
) {
    let readme = include_str!("../README.md")
        .lines()
        .collect::<Vec<_>>()
        .join(line_ending);
    // Git checkouts may use CRLF on Windows; the markers use logical lines.
    let readme = readme.replace("\r\n", "\n");
    let example = without_indentation(source);
    let marker = format!("<!-- example: {name} -->\n```rust\n");
    let section = readme
        .split_once(&marker)
        .expect("README must contain the named snippet")
        .1;
    let (snippet, _) = section
        .split_once("\n```\n<!-- endexample -->")
        .expect("Example snippets must have a closing marker");
    // Excerpts may omit unrelated declarations, but their remaining sections
    // must still appear verbatim and in order in the runnable example.
    let mut remaining = example.as_str();
    for excerpt in snippet.split("// ...") {
        let excerpt = without_indentation(excerpt).trim().to_owned();
        if excerpt.is_empty() {
            continue;
        }
        remaining = remaining
            .split_once(&excerpt)
            .unwrap_or_else(|| panic!("Refresh the {name} snippet from its example file"))
            .1;
    }
}
