//! 释义表：`词\t[词性. ]译文[|读音]\t…` 的 TSV，或打包好的 `.qj`（mmap 零拷贝，启动不用解析 20 多万行）。

mod entry_record;
mod sense_record;
mod storage;

use std::collections::HashMap;
use std::path::Path;

use qingjian_core::{Language, PartOfSpeech, Sense, Translation, Translator};
use qingjian_format::{
    Container, FORMAT_VERSION, GLOSSARY_MULTISENSE_VERSION, Kind, Metadata, Table, Writer, hash,
};

use crate::error::GlossaryError;
use entry_record::EntryRecord;
use sense_record::SenseRecord;
use storage::Storage;

/// `.qj` 里的分节：字符串 arena、词条表、释义表、哈希索引。
const TEXT_TAG: [u8; 4] = *b"TEXT";
const ENTRIES_TAG: [u8; 4] = *b"ENTR";
const SENSES_TAG: [u8; 4] = *b"SENS";
const HASH_TAG: [u8; 4] = *b"HASH";

/// 释义表，一个实例对应一种学习语言。
#[derive(Debug)]
pub struct Glossary {
    /// 本表的学习语言。
    language: Language,

    /// 词 → 译文。
    storage: Storage,
}

impl Glossary {
    pub fn empty(language: Language) -> Self {
        Self {
            language,
            storage: Storage::Owned(HashMap::new()),
        }
    }

    pub fn parse(language: Language, source: &str) -> Result<Self, GlossaryError> {
        let mut entries: HashMap<String, Translation> = HashMap::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (text, translation) = parse_entry(language, line, index + 1)?;
            entries.insert(text.to_owned(), translation);
        }
        tracing::debug!(
            language = language.code(),
            entries = entries.len(),
            "释义表加载完成"
        );
        Ok(Self {
            language,
            storage: Storage::Owned(entries),
        })
    }

    /// 按文件内容选加载方式：`.qj` 直接映射，否则当 TSV 解析。
    pub fn from_path(language: Language, path: impl AsRef<Path>) -> Result<Self, GlossaryError> {
        let path = path.as_ref();
        if Container::is_qj(path) {
            Self::open_qj(language, path)
        } else {
            Self::parse(language, &std::fs::read_to_string(path)?)
        }
    }

    /// 打开 `.qj` 释义表，逐条校验偏移落在 arena 内、在字符边界上。
    pub fn open_qj(language: Language, path: &Path) -> Result<Self, GlossaryError> {
        let container = Container::open(path, Kind::Glossary)?;
        let text = container.text(TEXT_TAG)?;
        let entries: Table<EntryRecord> = container.table(ENTRIES_TAG)?;
        let senses: Table<SenseRecord> = container.table(SENSES_TAG)?;
        let index: Table<u32> = container.table(HASH_TAG)?;
        let in_arena = |start: u32, len: u16| {
            let start = start as usize;
            start
                .checked_add(usize::from(len))
                .is_some_and(|end| text.get(start..end).is_some())
        };
        if entries.len() > u32::MAX as usize || senses.len() > u32::MAX as usize {
            return Err(GlossaryError::Corrupt(
                "entry or sense table exceeds 32-bit indices",
            ));
        }
        for entry in entries.iter() {
            if !in_arena(entry.word_start, entry.word_len)
                || entry.sense_count == 0
                || (entry.sense_start as usize)
                    .checked_add(usize::from(entry.sense_count))
                    .is_none_or(|end| end > senses.len())
            {
                return Err(GlossaryError::Corrupt(
                    "entry points outside the arena or sense table",
                ));
            }
        }
        if container.format_version() == FORMAT_VERSION
            && entries.iter().any(|entry| entry.sense_count > 2)
        {
            return Err(GlossaryError::Corrupt(
                "more than two senses require glossary format v2",
            ));
        }
        for sense in senses.iter() {
            if !in_arena(sense.text_start, sense.text_len)
                || !in_arena(sense.reading_start, sense.reading_len)
                || !in_arena(sense.pos_start, sense.pos_len)
            {
                return Err(GlossaryError::Corrupt("sense points outside the arena"));
            }
        }
        if !hash::is_valid(&index, entries.len()) {
            return Err(GlossaryError::Corrupt(
                "hash index does not match the entry table",
            ));
        }
        tracing::debug!(
            language = language.code(),
            entries = entries.len(),
            name = %container.metadata().name,
            "释义表已映射"
        );
        Ok(Self {
            language,
            storage: Storage::Mapped {
                text,
                entries,
                senses,
                index,
            },
        })
    }

    /// 写成 `.qj`：词按字节序排，字符串进一个 arena，词条 / 释义各一张定长表，外加哈希索引。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), GlossaryError> {
        if self.len() > u32::MAX as usize {
            return Err(GlossaryError::Limit("more than u32::MAX entries"));
        }
        let mut words: Vec<(&str, Translation)> = self.all_entries();
        words.sort_by(|a, b| a.0.cmp(b.0));
        let mut arena = String::new();
        let push = |arena: &mut String, s: &str| -> Result<(u32, u16), GlossaryError> {
            let start = u32::try_from(arena.len())
                .map_err(|_| GlossaryError::Limit("string arena exceeds u32::MAX bytes"))?;
            let len = u16::try_from(s.len())
                .map_err(|_| GlossaryError::Limit("field exceeds u16::MAX UTF-8 bytes"))?;
            if arena
                .len()
                .checked_add(s.len())
                .is_none_or(|end| end > u32::MAX as usize)
            {
                return Err(GlossaryError::Limit("string arena exceeds u32::MAX bytes"));
            }
            arena.push_str(s);
            Ok((start, len))
        };
        let mut entries: Vec<EntryRecord> = Vec::with_capacity(words.len());
        let mut senses: Vec<SenseRecord> = Vec::new();
        for (word, translation) in &words {
            validate_entry(word, translation.senses())?;
            if senses
                .len()
                .checked_add(translation.senses().len())
                .is_none_or(|end| end > u32::MAX as usize)
            {
                return Err(GlossaryError::Limit("sense table exceeds u32::MAX records"));
            }
            let (word_start, word_len) = push(&mut arena, word)?;
            let sense_start = u32::try_from(senses.len())
                .map_err(|_| GlossaryError::Limit("sense table exceeds u32::MAX records"))?;
            for sense in translation.senses() {
                let (text_start, text_len) = push(&mut arena, &sense.text)?;
                let (reading_start, reading_len) =
                    push(&mut arena, sense.reading.as_deref().unwrap_or(""))?;
                let (pos_start, pos_len) = push(
                    &mut arena,
                    sense.part_of_speech.map_or("", PartOfSpeech::abbreviation),
                )?;
                senses.push(SenseRecord {
                    text_start,
                    reading_start,
                    pos_start,
                    text_len,
                    reading_len,
                    pos_len,
                    reserved: 0,
                });
            }
            entries.push(EntryRecord {
                word_start,
                sense_start,
                word_len,
                sense_count: u16::try_from(translation.senses().len())
                    .map_err(|_| GlossaryError::Limit("more than u16::MAX senses per entry"))?,
            });
        }
        let index = hash::build(entries.len(), |id| {
            let e = &entries[id as usize];
            &arena[e.word_start as usize..e.word_start as usize + usize::from(e.word_len)]
        });
        let metadata = Metadata {
            entries: entries.len() as u64,
            ..metadata.clone()
        };
        let (entries, senses, index) = (
            Table::Owned(entries),
            Table::Owned(senses),
            Table::Owned(index),
        );
        let version = if words
            .iter()
            .any(|(_, translation)| translation.senses().len() > 2)
        {
            GLOSSARY_MULTISENSE_VERSION
        } else {
            FORMAT_VERSION
        };
        Writer::new(Kind::Glossary, &metadata)?
            .format_version(version)?
            .section(TEXT_TAG, arena.as_bytes())
            .section(ENTRIES_TAG, entries.as_bytes())
            .section(SENSES_TAG, senses.as_bytes())
            .section(HASH_TAG, index.as_bytes())
            .write_to(path)?;
        Ok(())
    }

    /// 全部条目（落盘用）。
    fn all_entries(&self) -> Vec<(&str, Translation)> {
        match &self.storage {
            Storage::Owned(map) => map.iter().map(|(k, v)| (k.as_str(), v.clone())).collect(),
            Storage::Mapped { entries, .. } => (0..entries.len() as u32)
                .filter_map(|id| Some((self.mapped_word(id)?, self.mapped_translation(id)?)))
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        match &self.storage {
            Storage::Owned(map) => map.len(),
            Storage::Mapped { entries, .. } => entries.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 映射表里第 `id` 个词。
    fn mapped_word(&self, id: u32) -> Option<&str> {
        let Storage::Mapped { text, entries, .. } = &self.storage else {
            return None;
        };
        let e = entries.get(id as usize)?;
        text.get(e.word_start as usize..e.word_start as usize + usize::from(e.word_len))
    }

    /// 映射表里第 `id` 个词的译文。
    fn mapped_translation(&self, id: u32) -> Option<Translation> {
        let Storage::Mapped {
            text,
            entries,
            senses,
            ..
        } = &self.storage
        else {
            return None;
        };
        let e = entries.get(id as usize)?;
        let slice = |start: u32, len: u16| -> Option<&str> {
            text.get(start as usize..start as usize + usize::from(len))
        };
        let start = e.sense_start as usize;
        let records = senses.get(start..start + usize::from(e.sense_count))?;
        let senses: Vec<Sense> = records
            .iter()
            .filter_map(|r| {
                let text = slice(r.text_start, r.text_len)?;
                let reading = slice(r.reading_start, r.reading_len)?;
                let pos = slice(r.pos_start, r.pos_len)?;
                Some(Sense {
                    part_of_speech: pos.parse::<PartOfSpeech>().ok(),
                    text: text.to_owned(),
                    reading: (!reading.is_empty()).then(|| reading.to_owned()),
                    fresh: false,
                })
            })
            .collect();
        Some(Translation::new(self.language, senses))
    }
}

/// TSV 与个人释义共用的容量检查；边界来自现有二进制字段，不按少量义项截断。
pub(crate) fn validate_entry(word: &str, senses: &[Sense]) -> Result<(), GlossaryError> {
    if word.len() > u16::MAX as usize {
        return Err(GlossaryError::Limit("word exceeds u16::MAX UTF-8 bytes"));
    }
    if senses.is_empty() || senses.len() > u16::MAX as usize {
        return Err(GlossaryError::Limit(
            "expected 1..=u16::MAX senses per entry",
        ));
    }
    if senses.iter().any(|sense| {
        sense.text.len() > u16::MAX as usize
            || sense
                .reading
                .as_ref()
                .is_some_and(|reading| reading.len() > u16::MAX as usize)
    }) {
        return Err(GlossaryError::Limit(
            "sense or reading exceeds u16::MAX UTF-8 bytes",
        ));
    }
    Ok(())
}

pub(crate) fn parse_entry(
    language: Language,
    line: &str,
    number: usize,
) -> Result<(&str, Translation), GlossaryError> {
    let error = |reason: &str| GlossaryError::Line {
        line: number,
        reason: reason.to_owned(),
    };
    let mut fields = line.split('\t').map(str::trim);
    let word = fields
        .next()
        .filter(|word| !word.is_empty())
        .ok_or_else(|| error("missing word"))?;
    let mut senses = Vec::new();
    for field in fields.filter(|field| !field.is_empty()) {
        if senses.len() == u16::MAX as usize {
            return Err(error("more than u16::MAX senses per entry"));
        }
        senses.push(parse_sense(field));
    }
    validate_entry(word, &senses).map_err(|limit| error(&limit.to_string()))?;
    Ok((word, Translation::new(language, senses)))
}

/// `v. develop` → 词性 + 译文；识别不出词性缩写时整段都是译文。译文后面 `|` 接的是读音（`n. 開発|かいはつ`）。
pub(crate) fn parse_sense(field: &str) -> Sense {
    let (field, reading) = match field.split_once('|') {
        Some((text, reading)) if !reading.trim().is_empty() => {
            (text.trim(), Some(reading.trim().to_owned()))
        }
        _ => (field, None),
    };
    if let Some((head, rest)) = field.split_once(' ')
        && head.ends_with('.')
        && let Ok(part_of_speech) = head.parse::<PartOfSpeech>()
    {
        return Sense {
            part_of_speech: Some(part_of_speech),
            text: rest.trim().to_owned(),
            reading,
            fresh: false,
        };
    }
    Sense {
        part_of_speech: None,
        text: field.to_owned(),
        reading,
        fresh: false,
    }
}

impl Translator for Glossary {
    fn language(&self) -> Language {
        self.language
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        match &self.storage {
            Storage::Owned(map) => map.get(text).cloned(),
            Storage::Mapped { index, .. } => {
                let id = hash::find(index, text, |id| self.mapped_word(id).unwrap_or(""))?;
                self.mapped_translation(id)
            }
        }
    }
}

#[cfg(test)]
mod tests;
