use super::*;

#[test]
fn parses_all_senses_with_optional_part_of_speech() {
    let glossary = Glossary::parse(
        Language::English,
        "# 注释\n开发\tv. develop\tn. development\tn. exploitation\n中文\tChinese language\n",
    )
    .unwrap();
    let translation = glossary.translate("开发").unwrap();
    assert_eq!(translation.language, Language::English);
    assert_eq!(translation.senses().len(), 3);
    assert_eq!(
        translation.senses()[1].part_of_speech,
        Some(PartOfSpeech::Noun)
    );
    assert_eq!(translation.senses()[1].text, "development");
    let plain = glossary.translate("中文").unwrap();
    assert_eq!(plain.senses()[0].part_of_speech, None);
    assert_eq!(plain.senses()[0].text, "Chinese language");
    assert!(glossary.translate("没有").is_none());
}

#[test]
fn japanese_senses_carry_kana_readings() {
    let glossary = Glossary::parse(
        Language::Japanese,
        "开发\tv. 開発する|かいはつする\tn. 開発\n私\t私|わたし\n",
    )
    .unwrap();
    let senses = glossary.translate("开发").unwrap().senses().to_vec();
    assert_eq!(senses[0].text, "開発する");
    assert_eq!(senses[0].reading.as_deref(), Some("かいはつする"));
    let segments = senses[0].furigana();
    assert_eq!(segments[0].text, "開発");
    assert_eq!(segments[0].reading.as_deref(), Some("かいはつ"));
    assert_eq!(segments[1].text, "する");
    assert_eq!(senses[1].reading, None);
    let plain = glossary.translate("私").unwrap();
    assert_eq!(plain.senses()[0].part_of_speech, None);
    assert_eq!(plain.senses()[0].reading.as_deref(), Some("わたし"));
}

#[test]
fn rejects_line_without_senses() {
    let error = Glossary::parse(Language::English, "开发\n").unwrap_err();
    assert!(matches!(error, GlossaryError::Line { line: 1, .. }));
}

#[test]
fn qj_round_trip_keeps_every_sense() {
    let source = "开发\tv. develop\tv. exploit\n你好\tint. hello\n開発\tn. 開発|かいはつ\n";
    let glossary = Glossary::parse(Language::English, source).unwrap();
    let dir = std::env::temp_dir().join("qingjian-glossary-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("glossary-{}.qj", std::process::id()));
    glossary
        .write_qj(
            &path,
            &Metadata {
                name: "测试释义".to_owned(),
                ..Metadata::default()
            },
        )
        .unwrap();
    let mapped = Glossary::from_path(Language::English, &path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(mapped.len(), 3);
    for word in ["开发", "你好", "開発"] {
        assert_eq!(mapped.translate(word), glossary.translate(word), "{word}");
    }
    assert_eq!(mapped.translate("没有"), None);
    let senses = mapped.translate("開発").unwrap();
    assert_eq!(senses.senses()[0].reading.as_deref(), Some("かいはつ"));
    assert_eq!(senses.senses()[0].part_of_speech, Some(PartOfSpeech::Noun));
}

#[test]
fn round_trip_one_two_three_and_many_senses_with_versions() {
    for count in [1, 2, 3, 129] {
        let fields: Vec<_> = (0..count)
            .map(|i| {
                format!(
                    "{} meaning{i}|reading{i}",
                    if i % 2 == 0 { "n." } else { "v." }
                )
            })
            .collect();
        let glossary =
            Glossary::parse(Language::English, &format!("word\t{}\n", fields.join("\t"))).unwrap();
        assert_eq!(glossary.translate("word").unwrap().senses().len(), count);
        let path = temp_path(&format!("count-{count}"));
        glossary.write_qj(&path, &Metadata::default()).unwrap();
        let container = Container::open(&path, Kind::Glossary).unwrap();
        assert_eq!(
            container.format_version(),
            if count > 2 {
                GLOSSARY_MULTISENSE_VERSION
            } else {
                FORMAT_VERSION
            }
        );
        drop(container);
        let mapped = Glossary::from_path(Language::English, &path).unwrap();
        assert_eq!(mapped.translate("word"), glossary.translate("word"));
        drop(mapped);
        if count > 2 {
            let mut bytes = std::fs::read(&path).unwrap();
            bytes[8..10].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
            std::fs::write(&path, bytes).unwrap();
            assert!(matches!(
                Glossary::from_path(Language::English, &path),
                Err(GlossaryError::Corrupt(_))
            ));
        }
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn storage_limits_reject_overflow_instead_of_truncating() {
    let maximum = (0..u16::MAX).map(|_| "n. x").collect::<Vec<_>>().join("\t");
    let glossary = Glossary::parse(Language::English, &format!("word\t{maximum}\n")).unwrap();
    assert_eq!(
        glossary.translate("word").unwrap().senses().len(),
        u16::MAX as usize
    );
    let path = temp_path("maximum");
    glossary.write_qj(&path, &Metadata::default()).unwrap();
    let mapped = Glossary::from_path(Language::English, &path).unwrap();
    assert_eq!(
        mapped.translate("word").unwrap().senses().len(),
        u16::MAX as usize
    );
    drop(mapped);
    std::fs::remove_file(path).unwrap();
    assert!(Glossary::parse(Language::English, &format!("word\t{maximum}\tn. extra\n")).is_err());
    let long = "x".repeat(u16::MAX as usize + 1);
    assert!(Glossary::parse(Language::English, &format!("{long}\tn. x\n")).is_err());
    assert!(Glossary::parse(Language::English, &format!("word\tn. {long}\n")).is_err());
    assert!(Glossary::parse(Language::English, &format!("word\tn. x|{long}\n")).is_err());
}

fn temp_path(label: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qingjian-multisense-{label}-{}-{nonce}.qj",
        std::process::id()
    ))
}

fn save_audit(name: &str, lines: &[String]) {
    if let Some(dir) = std::env::var_os("QINGJIAN_QA_REPORT_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), lines.join("\n") + "\n").unwrap();
    }
}

#[test]
fn every_real_source_glossary_round_trips_without_losing_senses() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/glossary");
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| {
            p.extension().is_some_and(|ext| ext == "tsv")
                && p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("glossary-")
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty());
    let mut audit = vec!["file\tentries\tsenses\tmaximum_per_word\tqj_version".to_owned()];
    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let language: Language = name.strip_prefix("glossary-").unwrap().parse().unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        let glossary = Glossary::parse(language, &source).unwrap();
        let packed = temp_path(name);
        glossary.write_qj(&packed, &Metadata::default()).unwrap();
        let mapped = Glossary::from_path(language, &packed).unwrap();
        assert_eq!(mapped.len(), glossary.len());
        let mut declared = HashMap::new();
        for line in source
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let mut fields = line.trim().split('\t');
            declared.insert(
                fields.next().unwrap(),
                fields.filter(|s| !s.trim().is_empty()).count(),
            );
        }
        let mut total = 0;
        let mut maximum = 0;
        for (word, expected) in glossary.all_entries() {
            let count = expected.senses().len();
            assert_eq!(count, declared[word], "{name}: {word}");
            assert_eq!(
                mapped.translate(word).as_ref(),
                Some(&expected),
                "{name}: {word}"
            );
            total += count;
            maximum = maximum.max(count);
        }
        let version = Container::open(&packed, Kind::Glossary)
            .unwrap()
            .format_version();
        audit.push(format!(
            "{name}.tsv\t{}\t{total}\t{maximum}\t{version}",
            glossary.len()
        ));
        drop(mapped);
        std::fs::remove_file(packed).unwrap();
    }
    save_audit("source-glossaries.tsv", &audit);
}

#[test]
#[ignore = "requires explicit public installed-asset directory; run separately with QINGJIAN_QA_LEGACY_DIR"]
fn legacy_packaged_glossaries_round_trip_without_changes() {
    let root = std::path::PathBuf::from(
        std::env::var_os("QINGJIAN_QA_LEGACY_DIR").expect("explicit QA directory required"),
    );
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| {
            p.extension().is_some_and(|ext| ext == "qj")
                && p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("glossary-")
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty());
    let mut audit = vec!["file\tentries\tsenses\tmaximum_per_word\tqj_version".to_owned()];
    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let language: Language = name.strip_prefix("glossary-").unwrap().parse().unwrap();
        let original = Container::open(&path, Kind::Glossary).unwrap();
        assert_eq!(original.format_version(), FORMAT_VERSION);
        let metadata = original.metadata().clone();
        drop(original);
        let legacy = Glossary::from_path(language, &path).unwrap();
        let packed = temp_path(name);
        legacy.write_qj(&packed, &metadata).unwrap();
        let mapped = Glossary::from_path(language, &packed).unwrap();
        assert_eq!(legacy.len(), mapped.len());
        let mut total = 0;
        let mut maximum = 0;
        for (word, expected) in legacy.all_entries() {
            assert_eq!(
                mapped.translate(word).as_ref(),
                Some(&expected),
                "{name}: {word}"
            );
            total += expected.senses().len();
            maximum = maximum.max(expected.senses().len());
        }
        assert_eq!(
            Container::open(&packed, Kind::Glossary)
                .unwrap()
                .format_version(),
            FORMAT_VERSION
        );
        audit.push(format!(
            "{name}.qj\t{}\t{total}\t{maximum}\t1",
            legacy.len()
        ));
        drop(mapped);
        std::fs::remove_file(packed).unwrap();
    }
    save_audit("legacy-glossaries.tsv", &audit);
}
