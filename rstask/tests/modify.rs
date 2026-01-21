mod common;

#[test]
fn test_modify_tasks_by_id() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "one", "+one"]);
    result.assert_success();

    let result = cmd.run(&["add", "two", "+two"]);
    result.assert_success();

    let result = cmd.run(&["add", "three", "+three"]);
    result.assert_success();

    let result = cmd.run(&["modify", "2", "3", "+extra"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();

    // Check that task three has both "three" and "extra" tags
    assert!(
        tasks[2].tags.contains(&"three".to_string())
            && tasks[2].tags.contains(&"extra".to_string()),
        "extra tag added to task three"
    );

    // Check that task two has both "two" and "extra" tags
    assert!(
        tasks[1].tags.contains(&"two".to_string()) && tasks[1].tags.contains(&"extra".to_string()),
        "extra tag added to task two"
    );

    // Check that task one only has "one" tag
    assert_eq!(tasks[0].tags, vec!["one"], "task 1 not modified");
}

#[test]
fn test_modify_tasks_in_context() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "one", "+one"]);
    result.assert_success();

    let result = cmd.run(&["add", "two", "+two"]);
    result.assert_success();

    let result = cmd.run(&["add", "three", "+three"]);
    result.assert_success();

    let result = cmd.run(&["context", "+three"]);
    result.assert_success();

    let result = cmd.run(&["modify", "+extra"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();

    // Tags should be sorted alphabetically, so ["extra", "three"]
    let expected_tags = vec!["extra".to_string(), "three".to_string()];
    assert_eq!(
        tasks[0].tags, expected_tags,
        "tags should have been modified"
    );
}
