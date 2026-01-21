mod common;

#[test]
fn test_next_search_word() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "one", "/", "alpha"]);
    result.assert_success();

    let result = cmd.run(&["add", "two"]);
    result.assert_success();

    // search something that doesn't exist
    let result = cmd.run(&["somethingRandom"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert!(
        tasks.is_empty(),
        "no tasks should be returned for a missing search term"
    );

    // search the summary of task two
    let result = cmd.run(&["two"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks[0].summary, "two", "search term should find a task");

    // search the notes field of task one
    let result = cmd.run(&["alpha"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(
        tasks[0].summary, "one",
        "string \"alpha\" is in a note for task one"
    );
}

#[test]
fn test_next_search_case_insensitive() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "one", "/", "alpha"]);
    result.assert_success();

    let result = cmd.run(&["add", "two"]);
    result.assert_success();

    // search should be case insensitive
    let result = cmd.run(&["TWO"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(
        tasks.len(),
        1,
        "string \"TWO\" should find task summary containing \"two\""
    );

    // case insensitive searching of notes field should work
    let result = cmd.run(&["ALPHA"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(
        tasks.len(),
        1,
        "string \"ALPHA\" should find notes field containing \"alpha\""
    );
}
