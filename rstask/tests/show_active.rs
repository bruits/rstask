mod common;

#[test]
fn test_show_active() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "+one", "one"]);
    result.assert_success();

    let result = cmd.run(&["start", "1"]);
    result.assert_success();

    let result = cmd.run(&["show-active"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "one", "one should be started");

    let result = cmd.run(&["stop", "1"]);
    result.assert_success();

    let result = cmd.run(&["show-active"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert!(tasks.is_empty(), "no tasks should be active");
}
