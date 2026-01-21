mod common;

#[test]
fn test_show_paused() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "+one", "one"]);
    result.assert_success();

    // "Paused" means tasks that were started, then stopped.
    let result = cmd.run(&["start", "1"]);
    result.assert_success();

    let result = cmd.run(&["stop", "1"]);
    result.assert_success();

    let result = cmd.run(&["show-paused"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "one");
}
