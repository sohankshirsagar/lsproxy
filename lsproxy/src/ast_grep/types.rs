use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    api_types::{FilePosition, Position, Symbol},
    utils::file_utils::absolute_path_to_relative_path_string,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepMatch {
    pub text: String,
    pub range: AstGrepRange,
    pub file: String,
    pub lines: String,
    pub char_count: CharCount,
    pub language: String,
    pub meta_variables: MetaVariables,
    pub rule_id: String,
    pub labels: Vec<Label>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepRange {
    pub byte_offset: ByteOffset,
    pub start: AstGrepPosition,
    pub end: AstGrepPosition,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ByteOffset {
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepPosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CharCount {
    pub leading: usize,
    pub trailing: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetaVariables {
    pub multi: MultiVariables,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MultiVariables {
    pub secondary: Vec<Secondary>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Secondary {
    pub text: String,
    pub range: AstGrepRange,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub text: String,
    pub range: AstGrepRange,
}

impl From<AstGrepMatch> for Symbol {
    fn from(ast_match: AstGrepMatch) -> Self {
        Symbol {
            name: ast_match.text,
            kind: ast_match.rule_id,
            identifier_position: FilePosition {
                path: absolute_path_to_relative_path_string(&PathBuf::from(ast_match.file)),
                position: Position {
                    line: ast_match.range.start.line as u32,
                    character: ast_match.range.start.column as u32,
                },
            },
        }
    }
}
