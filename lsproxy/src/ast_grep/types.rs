use std::path::PathBuf;
use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy;
use log::debug;

use serde::{Deserialize, Serialize};

static CALLABLE_TYPES: Lazy<HashMap<&'static str, HashSet<&'static str>>> = Lazy::new(|| {
        let mut m = HashMap::new();
        
        // C++
        let mut cpp = HashSet::new();
        cpp.insert("function-declaration");
        cpp.insert("function-definition");
        cpp.insert("class");
        m.insert("Cpp", cpp);

        // Go
        let mut go = HashSet::new();
        go.insert("function");
        go.insert("method");
        m.insert("Go", go);

        // Java
        let mut java = HashSet::new();
        java.insert("method");
        java.insert("class");
        m.insert("Java", java);

        // JavaScript
        let mut javascript = HashSet::new();
        javascript.insert("function");
        javascript.insert("method");
        javascript.insert("class");
        m.insert("JavaScript", javascript);

        // PHP
        let mut php = HashSet::new();
        php.insert("function");
        php.insert("method");
        php.insert("class");
        m.insert("Php", php);

        // Python
        let mut python = HashSet::new();
        python.insert("function");
        python.insert("class");
        m.insert("Python", python);

        // Rust
        let mut rust = HashSet::new();
        rust.insert("function");
        m.insert("Rust", rust);

        // TypeScript/TSX
        let mut typescript = HashSet::new();
        typescript.insert("function");
        typescript.insert("method");
        typescript.insert("class");
        m.insert("TypeScript", typescript.clone());
        m.insert("Tsx", typescript);

        m
    }
);

use crate::{
    api_types::{FilePosition, FileRange, Identifier, Position, Symbol},
    utils::file_utils::absolute_path_to_relative_path_string,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepMatch {
    pub text: String,
    range: AstGrepRange,
    pub file: String,
    pub lines: String,
    pub char_count: CharCount,
    pub language: String,
    pub meta_variables: MetaVariables,
    pub rule_id: String,
    pub labels: Option<Vec<Label>>,
}

impl AstGrepMatch {
    pub fn get_source_code(&self) -> String {
        if let Some(context) = &self.meta_variables.single.context {
            context.text.clone()
        } else {
            self.text.clone()
        }
    }

    pub fn get_context_range(&self) -> AstGrepRange {
        if let Some(context) = &self.meta_variables.single.context {
            context.range.clone()
        } else {
            self.range.clone()
        }
    }

    pub fn get_identifier_range(&self) -> AstGrepRange {
        self.meta_variables.single.name.range.clone()
    }

    pub fn contains(&self, other: &AstGrepMatch) -> bool {
        self.file == other.file
            && self.get_context_range().start.line <= other.get_context_range().start.line
            && self.get_context_range().end.line >= other.get_context_range().end.line
            && (self.get_context_range().start.line != other.get_context_range().start.line || self.get_context_range().start.column <= other.get_context_range().start.column)
            && (self.get_context_range().end.line != other.get_context_range().end.line || self.get_context_range().end.column >= other.get_context_range().end.column)
    }

    pub fn contains_location(&self, loc: &lsp_types::Location) -> bool {
        self.file == loc.uri.path()
            && self.get_context_range().start.line <= loc.range.start.line
            && self.get_context_range().end.line >= loc.range.end.line
            && (self.get_context_range().start.line != loc.range.start.line || self.get_context_range().start.column <= loc.range.start.character)
            && (self.get_context_range().end.line != loc.range.end.line || self.get_context_range().end.column >= loc.range.end.character)

    }

    pub fn contains_locationlink(&self, link: &lsp_types::LocationLink) -> bool {
        self.file == link.target_uri.path()
            && self.get_context_range().start.line <= link.target_range.start.line
            && self.get_context_range().end.line >= link.target_range.end.line
            && (self.get_context_range().start.line != link.target_range.start.line || self.get_context_range().start.column <= link.target_range.start.character)
            && (self.get_context_range().end.line != link.target_range.end.line || self.get_context_range().end.column >= link.target_range.end.character)
    }

    pub fn is_callable(&self) -> bool {
        if let Some(types) = CALLABLE_TYPES.get(self.language.as_str()) {
            types.contains(self.rule_id.as_str())
        } else {
            false
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepRange {
    pub byte_offset: ByteOffset,
    pub start: AstGrepPosition,
    pub end: AstGrepPosition,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ByteOffset {
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CharCount {
    pub leading: usize,
    pub trailing: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetaVariables {
    pub single: SingleVariable,
    pub multi: MultiVariables,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SingleVariable {
    #[serde(rename = "NAME")]
    pub name: MetaVariable,
    #[serde(rename = "CONTEXT")]
    pub context: Option<MetaVariable>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MultiVariables {
    pub secondary: Option<Vec<MetaVariable>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetaVariable {
    pub text: String,
    pub range: AstGrepRange,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub text: String,
    pub range: AstGrepRange,
}

impl From<&AstGrepMatch> for lsp_types::Position {
    fn from(ast_match: &AstGrepMatch) -> Self {
        Self {
            line: ast_match.range.start.line as u32,
            character: ast_match.range.start.column as u32,
        }
    }
}

impl From<AstGrepMatch> for Symbol {
    fn from(ast_match: AstGrepMatch) -> Self {
        assert!(ast_match.rule_id != "all-identifiers");
        let path = absolute_path_to_relative_path_string(&PathBuf::from(ast_match.file.clone()));
        let match_range = ast_match.get_context_range();
        Symbol {
            name: ast_match.meta_variables.single.name.text.clone(),
            kind: ast_match.rule_id.clone(),
            identifier_position: FilePosition {
                path: path.clone(),
                position: Position {
                    line: ast_match.range.start.line as u32,
                    character: ast_match.range.start.column as u32,
                },
            },
            range: FileRange {
                path: path.clone(),
                start: Position {
                    line: match_range.start.line as u32,
                    character: 0, // TODO: this is not technically true, we're returning the whole line for consistency
                },
                end: Position {
                    line: match_range.end.line as u32,
                    character: match_range.end.column as u32,
                },
            },
        }
    }
}

impl From<AstGrepMatch> for Identifier {
    fn from(ast_match: AstGrepMatch) -> Self {
        let path = absolute_path_to_relative_path_string(&PathBuf::from(ast_match.file.clone()));
        let match_range = ast_match.get_context_range();
        let kind = match ast_match.rule_id.as_str() {
            "all-identifiers" => None,
            _ => Some(ast_match.rule_id),
        };

        Identifier {
            name: ast_match.meta_variables.single.name.text.clone(),
            kind,
            range: FileRange {
                path: path.clone(),
                start: Position {
                    line: match_range.start.line as u32,
                    character: match_range.start.column as u32,
                },
                end: Position {
                    line: match_range.end.line as u32,
                    character: match_range.end.column as u32,
                },
            },
        }
    }
}
