// This is a metapackage for tests
// Re-export crates as modules

// Test helpers and utilities
pub mod test_helpers {
    #[cfg(test)]
    mod tests {
        #[test]
        fn simple_test() {
            assert!(true);
        }
    }
}

// Re-export common error type for convenience
pub use common::error::Result;