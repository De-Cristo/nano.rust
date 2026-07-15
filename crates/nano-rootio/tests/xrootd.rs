#![cfg(feature = "xrootd")]

use nano_rootio::Source;

#[test]
fn xrootd_source_rejects_non_xrootd_urls_before_opening_a_connection() {
    let error = Source::xrootd("https://example.invalid/input.root").expect_err("invalid scheme");
    assert!(
        error
            .to_string()
            .contains("requires root:// or roots:// URL"),
        "unexpected error: {error}"
    );
}
