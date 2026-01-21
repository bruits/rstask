mod common;

#[test]
fn test_show_unorganised() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "two", "+two"]);
    result.assert_success();

    let result = cmd.run(&["show-unorganised"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "one", "task one has no tags or projects");
}
