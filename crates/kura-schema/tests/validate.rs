use kura_schema::{LoadError, Part, parse_part};

const MINHASH: &str = include_str!("../../../registry/minhash.json");

fn minhash() -> Part {
    parse_part("minhash.json", MINHASH).expect("registry/minhash.json is valid")
}

fn fields(part: &Part) -> Vec<String> {
    part.validate().into_iter().map(|i| i.field).collect()
}

#[test]
fn measured_part_is_valid() {
    let part = minhash();
    assert!(part.validate().is_empty());
    assert!(!part.sample, "minhash is measured by verify/run.py");
}

#[test]
fn measured_part_needs_environment() {
    let mut part = minhash();
    part.environment = None;
    assert_eq!(fields(&part), ["environment"]);
    part.sample = true;
    assert!(
        part.validate().is_empty(),
        "sample parts may omit the environment"
    );
}

#[test]
fn rejects_passed_greater_than_cases() {
    let mut part = minhash();
    part.verification.passed = part.verification.cases + 1;
    assert_eq!(fields(&part), ["verification.passed"]);
}

#[test]
fn rejects_unsorted_benchmarks() {
    let mut part = minhash();
    part.benchmarks.swap(0, 1);
    assert!(fields(&part).contains(&"benchmarks[1].input_size".to_owned()));
}

#[test]
fn rejects_non_positive_times() {
    let mut part = minhash();
    part.benchmarks[0].rust_ms = 0.0;
    assert_eq!(fields(&part), ["benchmarks[0].rust_ms"]);
}

#[test]
fn rejects_bad_names_and_versions() {
    let mut part = minhash();
    part.name = "MinHash".into();
    part.version = "1.0".into();
    assert_eq!(fields(&part), ["name", "version"]);

    part.name = "index".into();
    part.version = "1.0.0".into();
    assert_eq!(fields(&part), ["name"]);
}

#[test]
fn rejects_duplicate_or_missing_shelves() {
    let mut part = minhash();
    part.shelves = vec![];
    assert_eq!(fields(&part), ["shelves"]);
    part.shelves = vec![kura_schema::Shelf::Ai, kura_schema::Shelf::Ai];
    assert_eq!(fields(&part), ["shelves"]);
}

#[test]
fn python_and_python_usage_go_together() {
    let mut part = minhash();
    part.python = Some(kura_schema::PythonPackage {
        package: "kura-rs".into(),
        import_path: "from kura_rs import minhash".into(),
    });
    part.usage.python = None;
    assert_eq!(fields(&part), ["usage.python"]);

    part.python = None;
    part.usage.python = Some("import kura_rs".into());
    assert_eq!(fields(&part), ["python"]);
}

#[test]
fn rejects_paths_outside_repository() {
    let mut part = minhash();
    part.rust.files.push("../secret.rs".into());
    assert_eq!(fields(&part), ["rust.files[2]"]);
}

#[test]
fn rejects_unknown_fields_and_shelves() {
    let edited = |edit: fn(&mut serde_json::Value)| {
        let mut json: serde_json::Value = serde_json::from_str(MINHASH).unwrap();
        edit(&mut json);
        json.to_string()
    };

    let unknown_field = edited(|j| j["extra"] = 1.into());
    assert!(matches!(
        parse_part("minhash.json", &unknown_field).unwrap_err()[..],
        [LoadError::Parse { .. }]
    ));

    let unknown_shelf = edited(|j| j["shelves"] = serde_json::json!(["web"]));
    assert!(matches!(
        parse_part("minhash.json", &unknown_shelf).unwrap_err()[..],
        [LoadError::Parse { .. }]
    ));
}

#[test]
fn translations_must_not_be_blank() {
    let mut part = minhash();
    part.translations = Some(kura_schema::Translations {
        ja: Some(kura_schema::PartTranslation {
            title: Some(" ".into()),
            description: None,
        }),
        ..Default::default()
    });
    assert_eq!(fields(&part), ["translations.ja.title"]);
}

#[test]
fn rejects_unknown_locales() {
    let json = MINHASH.replacen(
        "\"translations\": {",
        "\"translations\": {\n    \"xx\": {},",
        1,
    );
    assert!(matches!(
        parse_part("minhash.json", &json).unwrap_err()[..],
        [LoadError::Parse { .. }]
    ));
}

#[test]
fn reports_missing_translations() {
    let mut part = minhash();
    assert!(part.missing_translations().is_empty());
    part.translations.as_mut().unwrap().ko = None;
    assert_eq!(part.missing_translations(), ["ko"]);
    part.translations = None;
    assert_eq!(
        part.missing_translations().len(),
        kura_schema::Translations::LOCALES.len()
    );
}

#[test]
fn file_name_must_match_name() {
    let errors = parse_part("other.json", MINHASH).unwrap_err();
    assert!(matches!(errors[..], [LoadError::FileName { .. }]));
}

#[test]
fn round_trips_through_json() {
    let part = minhash();
    let json = serde_json::to_string(&part).unwrap();
    assert_eq!(serde_json::from_str::<Part>(&json).unwrap(), part);
}

#[test]
fn max_speedup_is_the_best_point() {
    let part = minhash();
    let best = part
        .benchmarks
        .iter()
        .map(|p| p.reference_ms / p.rust_ms)
        .fold(0.0, f64::max);
    assert_eq!(part.max_speedup(), Some(best));
}
