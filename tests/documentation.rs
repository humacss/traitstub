use rstest::rstest;

fn without_indentation(text: &str) -> String {
    text.lines().map(str::trim).collect::<Vec<_>>().join("\n")
}

#[rstest]
#[case::dependencies("dependencies")]
#[case::consumer("consumer")]
#[case::shared_stubs("shared-stubs")]
#[case::individual_override("override")]
fn readme_snippets_match_the_runnable_example(#[case] name: &str) {
    let readme = include_str!("../README.md");
    let example = without_indentation(include_str!("../examples/common.rs"));
    let marker = format!("<!-- example: {name} -->\n```rust\n");
    let section = readme
        .split_once(&marker)
        .expect("README must contain the named snippet")
        .1;
    let (snippet, _) = section
        .split_once("\n```\n<!-- endexample -->")
        .expect("Example snippets must have a closing marker");
    assert!(
        example.contains(&without_indentation(snippet)),
        "Refresh the {name} snippet from examples/common.rs"
    );
}
