//! 写出保留原生选项与注释的 Windows 离线英译中配置；只写指定的新文件，不启动输入法。

use std::path::PathBuf;

use qingjian_platform::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(std::env::args().nth(1).ok_or("missing output path")?);
    if !Config::write_template_if_missing(&path)? {
        return Err("refusing to overwrite an existing configuration".into());
    }
    // 不调用 Config::load，避免为了生成公开预设而解析环境中的密钥。
    let mut expected: Config = toml::from_str(&std::fs::read_to_string(&path)?)?;
    for (section, key, value) in [
        ("general", "english_to_chinese", true),
        ("general", "default_english", true),
        ("general", "input_log", false),
        ("predict", "enabled", false),
        ("predict", "automatic", false),
        ("predict", "sentence_translation", false),
        ("predict", "vocabulary_translation", false),
        ("update", "check", false),
    ] {
        Config::set_bool(&path, section, key, value)?;
    }
    Config::set_value(&path, "predict", "api_key_env", "")?;
    expected.general.english_to_chinese = true;
    expected.general.default_english = true;
    expected.general.input_log = false;
    expected.predict.enabled = false;
    expected.predict.automatic = false;
    expected.predict.sentence_translation = false;
    expected.predict.vocabulary_translation = false;
    expected.predict.api_key_env.clear();
    expected.update.check = false;
    expected
        .apps
        .english_candidates_on
        .push("ChatGPT.exe".into());
    Config::set_array(
        &path,
        "apps",
        "english_candidates_on",
        &expected.apps.english_candidates_on,
    )?;
    let actual: Config = toml::from_str(&std::fs::read_to_string(&path)?)?;
    if actual != expected {
        return Err("configuration changed outside the explicit offline overrides".into());
    }
    println!("{}", serde_json::to_string_pretty(&actual)?);
    Ok(())
}
