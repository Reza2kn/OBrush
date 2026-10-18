#[path = "common/paint_fixture.rs"]
mod fixture;

/// Run explicitly against immutable pre-channel source; this command produces
/// a historical oracle, and is not an ordinary test run's golden-data updater.
#[test]
#[ignore = "explicit frozen-source legacy snapshot generation only"]
fn write_frozen_legacy_rgb_bits() {
    let path = std::env::var("OBRUSH_PAINT_LEGACY_OUTPUT").expect("Explicit output path required");
    let cases = fixture::legacy_cases();
    assert_eq!(cases.as_array().unwrap().len(), 32);
    std::fs::write(path, serde_json::to_vec(&cases).unwrap()).unwrap();
}
