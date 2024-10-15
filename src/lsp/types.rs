use serde::{Deserialize, Serialize};
use std::hash::Hash;
use strum_macros::{Display, EnumString};
use lsp_types::{DocumentSymbolResponse, SymbolKind, DocumentSymbol};

pub const MOUNT_DIR: &str = "/mnt/repo";

#[derive(
    Debug,
    EnumString,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
#[strum(serialize_all = "lowercase")]
pub enum SupportedLSP {
    Python,
    TypeScriptJavaScript,
    Rust,
}

 #[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
 pub struct SimplifiedSymbol {
     pub name: String,
     pub kind: String,
     pub line: u32,
     pub character: u32,
 }

 #[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CustomDocumentSymbolResponse {
    raw_response: serde_json::Value,
    simplified: Vec<SimplifiedSymbol>,
}

impl From<DocumentSymbolResponse> for CustomDocumentSymbolResponse {
    fn from(response: DocumentSymbolResponse) -> Self {
        let raw_response = serde_json::to_value(&response).unwrap_or_default();
        let simplified = match response {
            DocumentSymbolResponse::Flat(symbols) => symbols
                .into_iter()
                .map(|symbol| SimplifiedSymbol {
                    name: symbol.name,
                    kind: symbol_kind_to_string(&symbol.kind).to_string(),
                    line: symbol.location.range.start.line,
                    character: symbol.location.range.start.character,
                })
                .collect(),
            DocumentSymbolResponse::Nested(symbols) => flatten_nested_symbols(symbols)
        };
        CustomDocumentSymbolResponse {
            raw_response,
            simplified,
        }
    }
}

fn flatten_nested_symbols(symbols: Vec<DocumentSymbol>) -> Vec<SimplifiedSymbol> {
    fn recursive_flatten(symbol: DocumentSymbol, result: &mut Vec<SimplifiedSymbol>) {
        result.push(SimplifiedSymbol {
            name: symbol.name,
            kind: symbol_kind_to_string(&symbol.kind).to_string(),
            line: symbol.selection_range.start.line,
            character: symbol.selection_range.start.character,
        });

        for child in symbol.children.unwrap_or_default() {
            recursive_flatten(child, result);
        }
    }

    let mut flattened = Vec::new();
    for symbol in symbols {
        recursive_flatten(symbol, &mut flattened);
    }
    flattened
}

fn symbol_kind_to_string(kind: &SymbolKind) -> &str {
    match kind {
        &SymbolKind::FILE => "file",
        &SymbolKind::MODULE => "module",
        &SymbolKind::NAMESPACE => "namespace",
        &SymbolKind::PACKAGE => "package",
        &SymbolKind::CLASS => "class",
        &SymbolKind::METHOD => "method",
        &SymbolKind::PROPERTY => "property",
        &SymbolKind::FIELD => "field",
        &SymbolKind::CONSTRUCTOR => "constructor",
        &SymbolKind::ENUM => "enum",
        &SymbolKind::INTERFACE => "interface",
        &SymbolKind::FUNCTION => "function",
        &SymbolKind::VARIABLE => "variable",
        &SymbolKind::CONSTANT => "constant",
        &SymbolKind::STRING => "string",
        &SymbolKind::NUMBER => "number",
        &SymbolKind::BOOLEAN => "boolean",
        &SymbolKind::ARRAY => "array",
        &SymbolKind::OBJECT => "object",
        &SymbolKind::KEY => "key",
        &SymbolKind::NULL => "null",
        &SymbolKind::ENUM_MEMBER => "enum_member",
        &SymbolKind::STRUCT => "struct",
        &SymbolKind::EVENT => "event",
        &SymbolKind::OPERATOR => "operator",
        &SymbolKind::TYPE_PARAMETER => "type_parameter",
        _ => "unknown", // Default case for any future additions
    }
}

