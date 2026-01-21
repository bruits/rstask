mod common;

#[test]
fn test_show_templates() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "two"]);
    result.assert_success();

    let result = cmd.run(&["template", "template1"]);
    result.assert_success();

    let result = cmd.run(&["show-templates"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "template1", "should be a template");
}
