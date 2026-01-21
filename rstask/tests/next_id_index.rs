mod common;

#[test]
fn test_next_by_id_index() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "two", "+two"]);
    result.assert_success();

    let result = cmd.run(&["1"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "one", "find task 1 by ID");
}

#[test]
fn test_next_by_id_index_outside_context() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "one", "+one"]);
    result.assert_success();

    let result = cmd.run(&["add", "two", "+two"]);
    result.assert_success();

    let result = cmd.run(&["context", "+one"]);
    result.assert_success();

    let result = cmd.run(&["2"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(
        tasks[0].summary, "two",
        "find task 2 by ID (context ignored with ID based addressing)"
    );

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].id, 1, "1 is the only ID in our current context");
}
