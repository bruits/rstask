mod common;

#[test]
fn test_next_tag_filter() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "+one", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "+one", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "+two", "two"]);
    result.assert_success();

    let result = cmd.run(&["next", "+one"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "one");

    let result = cmd.run(&["next", "+two"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "two");
}

#[test]
fn test_next_multiple_tag_filter() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "+one", "one-alpha"]);
    result.assert_success();

    let result = cmd.run(&["add", "+one", "+beta", "one-beta"]);
    result.assert_success();

    let result = cmd.run(&["add", "+two", "two"]);
    result.assert_success();

    let result = cmd.run(&["next", "+one", "+beta"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "one-beta");
    assert_eq!(tasks.len(), 1);
}

#[test]
fn test_next_project_filter() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "+one", "one"]);
    result.assert_success();

    let result = cmd.run(&["add", "+two", "project:house", "two"]);
    result.assert_success();

    let result = cmd.run(&["next", "project:house"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "two");

    let result = cmd.run(&["project:house"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "two");

    let result = cmd.run(&["-project:house"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "one");
}
