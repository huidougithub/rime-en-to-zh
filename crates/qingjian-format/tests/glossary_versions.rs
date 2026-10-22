//! 多义释义表显式版本与基础数据旧版本的边界。
use qingjian_format::{
    Container, FORMAT_VERSION, FormatError, GLOSSARY_MULTISENSE_VERSION, Kind, Metadata, Writer,
};

#[test]
fn only_glossary_can_select_the_multisense_version() {
    for version in [FORMAT_VERSION, GLOSSARY_MULTISENSE_VERSION] {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "qingjian-version-{version}-{}-{nonce}.qj",
            std::process::id()
        ));
        Writer::new(Kind::Glossary, &Metadata::default())
            .unwrap()
            .format_version(version)
            .unwrap()
            .write_to(&path)
            .unwrap();
        let container = Container::open(&path, Kind::Glossary).unwrap();
        assert_eq!(container.format_version(), version);
        drop(container);
        if version == FORMAT_VERSION {
            let mut bytes = std::fs::read(&path).unwrap();
            bytes[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
            std::fs::write(&path, bytes).unwrap();
            assert!(matches!(
                Container::open(&path, Kind::Glossary),
                Err(FormatError::Truncated)
            ));
        }
        std::fs::remove_file(path).unwrap();
    }
    assert!(
        Writer::new(Kind::Dictionary, &Metadata::default())
            .unwrap()
            .format_version(GLOSSARY_MULTISENSE_VERSION)
            .is_err()
    );
    assert!(
        Writer::new(Kind::Glossary, &Metadata::default())
            .unwrap()
            .format_version(3)
            .is_err()
    );
}
