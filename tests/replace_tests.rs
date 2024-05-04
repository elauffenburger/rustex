use std::sync;

use rustex::{executor::Executor, parser::Parser, replace::ReplaceSpec};
use tracing_subscriber::EnvFilter;

const TEST_INIT: sync::Once = sync::Once::new();

fn init_tests() {
    TEST_INIT.call_once(|| {
        tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).with_test_writer().init();
    });
}

async fn run_test(pattern: &str, input: &str, replace_pattern: &str) -> Result<String, Box<String>> {
    init_tests();

    let parse_res = Parser::new()
        .parse_str(pattern)
        .map_err(|err| format!("failed to parse: {:?}", err))?;

    let exec_res = Executor::new()
        .exec(&parse_res, input)
        .await
        .map_err(|err| format!("failed to exec: {:?}", err))?
        .ok_or_else(|| format!("empty exec result"))?;

    let spec = ReplaceSpec::parse_str(replace_pattern);

    spec.perform_replace(input, &exec_res)
        .ok_or_else(|| Box::new(format!("failed to perform replace or empty replace pattern")))
}

#[tokio::test]
async fn test_replace_basic() {
    let result = run_test("(he)llo wo(r)ld!", "hello world!", "$1llo $2ust!")
        .await
        .unwrap();

    insta::assert_debug_snapshot!(result);
}
