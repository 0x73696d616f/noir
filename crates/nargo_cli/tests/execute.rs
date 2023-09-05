#[allow(unused_imports)]
#[cfg(test)]
mod tests {
    // Some of these imports are consumed by the injected tests
    use assert_cmd::prelude::*;
    use predicates::prelude::*;
    use tempdir::TempDir;

    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    use super::*;

    test_binary::build_test_binary_once!(
        mock_backend,
        "../acvm_backend_barretenberg/test-binaries"
    );

    // include tests generated by `build.rs`
    include!(concat!(env!("OUT_DIR"), "/execute.rs"));
}
