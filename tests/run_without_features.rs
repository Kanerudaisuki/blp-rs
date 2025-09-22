#![cfg(all(not(feature = "cli"), not(feature = "ui")))]

use blp_rs::error::args::ArgVal;
use blp_rs::run::run;

#[test]
fn run_without_required_features_returns_error() {
    let err = run().expect_err("run() should error with required features disabled");
    assert_eq!(err.key, "runtime-features-disabled");
    let features = err
        .args
        .get("features")
        .and_then(|val| match val {
            ArgVal::Str(s) => Some(s.as_ref()),
            _ => None,
        });
    assert_eq!(features, Some("cli, ui"));
}
