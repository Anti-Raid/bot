#[allow(unused_imports)] // They aren't unused
use serenity::all::{FullEvent, HttpBuilder};
use silverpelt::Error;

fn modules() -> Vec<Box<dyn modules::modules::Module>> {
    bot_modules_default::modules()
}

pub async fn run_tester() {
    test_module_parse();
    check_modules_test()
        .await
        .expect("Failed to check modules test");
}

pub fn test_module_parse() {
    let _ = modules();
}

pub async fn check_modules_test() -> Result<(), Error> {
    // Check for env var CHECK_MODULES_TEST_ENABLED
    if std::env::var("CHECK_MODULES_TEST_ENABLED").unwrap_or_default() == "true" {
        return Ok(());
    }

    // Set current directory to ../../
    let current_dir = std::env::current_dir().unwrap();

    if current_dir.ends_with("services/rust.bot") {
        std::env::set_current_dir("../../")?;
    }

    for module in modules() {
        module.validate()?;
    }

    Ok(())
}
