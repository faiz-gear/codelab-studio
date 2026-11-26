use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::LanguageFeature;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguagePosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageRange {
    pub start: LanguagePosition,
    pub end: LanguagePosition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageDiagnostic {
    pub range: LanguageRange,
    pub message: String,
    pub severity: Option<u8>,
    pub source: Option<String>,
    pub code: Option<DiagnosticCode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DiagnosticCode {
    Number(i64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LanguageMarkup {
    Text(String),
    Markup { kind: String, value: String },
    Code { language: String, value: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageTextEdit {
    pub new_text: String,
    pub range: LanguageRange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionTextEdit {
    pub new_text: String,
    pub range: Option<LanguageRange>,
    pub insert: Option<LanguageRange>,
    pub replace: Option<LanguageRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCompletionItem {
    pub label: String,
    pub kind: Option<u8>,
    pub detail: Option<String>,
    pub documentation: Option<LanguageMarkup>,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
    pub insert_text: Option<String>,
    pub insert_text_format: Option<u8>,
    pub text_edit: Option<CompletionTextEdit>,
    #[serde(default)]
    pub additional_text_edits: Vec<LanguageTextEdit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCompletion {
    pub items: Vec<LanguageCompletionItem>,
    pub is_incomplete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageHover {
    pub contents: Vec<LanguageMarkup>,
    pub range: Option<LanguageRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum LanguageFeatureResult {
    Completion(LanguageCompletion),
    Hover(LanguageHover),
}

impl LanguageFeatureResult {
    pub(super) fn parse(feature: LanguageFeature, value: Value) -> Option<Self> {
        if value.is_null() {
            return None;
        }
        match feature {
            LanguageFeature::Completion => {
                let is_incomplete = value["isIncomplete"].as_bool().unwrap_or(false);
                let list = match value {
                    Value::Array(list) => list,
                    Value::Object(mut fields) => match fields.remove("items")? {
                        Value::Array(list) => list,
                        _ => return None,
                    },
                    _ => return None,
                };
                Some(Self::Completion(LanguageCompletion {
                    items: decode_list(list),
                    is_incomplete,
                }))
            }
            LanguageFeature::Hover => {
                let Value::Object(mut fields) = value else {
                    return None;
                };
                let list = match fields.remove("contents")? {
                    Value::Array(list) => list,
                    content => vec![content],
                };
                let contents = decode_list(list);
                let range = fields
                    .remove("range")
                    .and_then(|range| serde_json::from_value(range).ok());
                Some(Self::Hover(LanguageHover { contents, range }))
            }
        }
    }
}

pub(super) fn decode_list<T: DeserializeOwned>(values: Vec<Value>) -> Vec<T> {
    values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect()
}
