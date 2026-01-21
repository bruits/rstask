mod common;

#[test]
fn test_show_resolved() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "+one", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "+two", "two"]);
    result.assert_success();

    let result = cmd.run(&["add", "three"]);
    result.assert_success();

    let result = cmd.run(&["1", "done"]);
    result.assert_success();

    let result = cmd.run(&["show-resolved"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "one", "one should be resolved");

    // Test the sorting of resolved tasks
    let result = cmd.run(&["3", "done"]);
    result.assert_success();

    let result = cmd.run(&["2", "done"]);
    result.assert_success();

    let result = cmd.run(&["show-resolved"]);
    result.assert_success();

    // sorting is ascending, so the most recently resolved tasks are shown last
    // (visible in terminal)
    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "one", "one should be resolved");

    // Check that resolved time is set
    assert!(
        tasks[0].resolved.is_some(),
        "resolved time should be set for resolved task"
    );
}
