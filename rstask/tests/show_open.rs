mod common;

#[test]
fn test_show_open() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "+one", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "+two", "two"]);
    result.assert_success();

    let result = cmd.run(&["show-open"]);
    result.assert_success();

    // Oldest tasks come first
    let tasks = result.parse_tasks();
    assert_eq!(tasks[1].summary, "two", "two should be sorted last");

    let result = cmd.run(&["context", "-one"]);
    result.assert_success();

    let result = cmd.run(&["show-open"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "two", "setting -one as a context");

    let result = cmd.run(&["2", "done"]);
    result.assert_success();

    let result = cmd.run(&["show-open"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 0, "no tasks open in this context");
}
