mod common;

#[test]
fn test_setting_tag_context() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "+one", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "+one", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "+two", "two"]);
    result.assert_success();

    let result = cmd.run(&["context", "+two"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "two", "setting +two as a context");

    let result = cmd.run(&["context", "-one"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "two", "setting -one as a context");
}

#[test]
fn test_setting_tag_and_project_context() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "+one", "+alpha", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "project:beta", "+two", "two"]);
    result.assert_success();

    let result = cmd.run(&["context", "project:beta"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "two", "setting project:beta as a context");

    let result = cmd.run(&["context", "project:beta", "+one"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert!(
        tasks.is_empty(),
        "no tasks within context project:beta +one"
    );
}

#[test]
fn test_context_from_env_var() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "+one", "+alpha", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "project:beta", "+two", "two"]);
    result.assert_success();

    let result = cmd.run(&["context", "project:beta"]);
    result.assert_success();

    // Create a custom command with DSTASK_CONTEXT environment variable
    let cmd_with_ctx = common::TestCmd::new_with_context(&_repo, "+one +alpha");

    let result = cmd_with_ctx.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(
        tasks[0].summary, "one",
        "'+one +alpha' context set by DSTASK_CONTEXT"
    );

    // Use original cmd without DSTASK_CONTEXT to verify on-disk context
    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "two", "project:beta is on-disk context");
}
