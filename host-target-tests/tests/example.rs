#[test]
fn example_test_pass() -> Result<(), String> {
    Ok(())
}

#[test]
fn example_test_fail() -> Result<(), String> {
    Err(String::from("Example failure"))
}