use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::path::PathBuf;
use strum_macros::{Display, EnumString};
use lsp_types::{DocumentSymbolResponse, GotoDefinitionResponse, SymbolKind, DocumentSymbol, LocationLink, Location};

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
pub struct SimplifiedLocation {
    pub uri: String,
    pub line: u32,
    pub character: u32,
}

 #[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
 pub struct SimplifiedSymbol {
     pub name: String,
     pub kind: String,
     pub line: u32,
     pub character: u32,
 }

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CustomGotoDefinitionResponse {
    raw_response: serde_json::Value,
    simplified: Vec<SimplifiedLocation>,
}

impl From<GotoDefinitionResponse> for CustomGotoDefinitionResponse {
    fn from(response: GotoDefinitionResponse) -> Self {
        let raw_response = serde_json::to_value(&response).unwrap_or_default();
        let simplified = match response {
            GotoDefinitionResponse::Scalar(location) => vec![simplify_location(location)],
            GotoDefinitionResponse::Array(locations) => locations.into_iter().map(simplify_location).collect(),
            GotoDefinitionResponse::Link(links) => links.into_iter().map(simplify_location_link).collect(),
        };
        CustomGotoDefinitionResponse {
            raw_response,
            simplified,
        }
    }
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

fn simplify_location(location: Location) -> SimplifiedLocation {
    SimplifiedLocation {
        uri: simplify_uri(location.uri),
        line: location.range.start.line,
        character: location.range.start.character,
    }
}

fn simplify_location_link(link: LocationLink) -> SimplifiedLocation {
    SimplifiedLocation {
        uri: simplify_uri(link.target_uri),
        line: link.target_range.start.line,
        character: link.target_range.start.character,
    }
}

fn simplify_uri(uri: lsp_types::Url) -> String {
    let path = uri.to_file_path().unwrap_or_else(|_| PathBuf::from(uri.path()));
    let current_dir = std::env::current_dir().unwrap_or_default();

    path.strip_prefix(&current_dir)
        .map(|relative| relative.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
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

