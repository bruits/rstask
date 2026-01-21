mod common;

#[test]
fn test_log() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "one"]);
    result.assert_success();

    let result = cmd.run(&["log", "two"]);
    result.assert_success();

    let result = cmd.run(&["show-resolved"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert!(
        !tasks.is_empty(),
        "Expected resolved tasks but got none. stdout: {}, stderr: {}",
        result.stdout(),
        result.stderr()
    );
    assert_eq!(tasks[0].summary, "two", "task two should be resolved");
}
