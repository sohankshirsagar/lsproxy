use log::debug;
use std::{error::Error, fs};
use tree_sitter::{Parser, Query, QueryCursor};

#[derive(Debug)]
pub struct SymbolOccurrence {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

pub fn find_symbol_occurrences(
    file_path: &str,
    symbol_name: &str,
) -> Result<Vec<SymbolOccurrence>, Box<dyn Error + Send + Sync>> {
    debug!(
        "Searching for symbol '{}' in file '{}'",
        symbol_name, file_path
    );

    let source_code = fs::read_to_string(file_path)?;
    debug!(
        "File content loaded, length: {} characters",
        source_code.len()
    );

    let mut parser = Parser::new();
    parser.set_language(tree_sitter_python::language())?;

    let tree = parser
        .parse(&source_code, None)
        .ok_or("Failed to parse the source code")?;
    debug!("Source code parsed successfully");

    let query = Query::new(
        tree_sitter_python::language(),
        &format!(r#"((identifier) @id (#eq? @id "{}"))"#, symbol_name),
    )?;
    debug!("Query created for symbol '{}'", symbol_name);

    let mut query_cursor = QueryCursor::new();
    let matches = query_cursor.matches(&query, tree.root_node(), source_code.as_bytes());

    let occurrences: Vec<SymbolOccurrence> = matches
        .flat_map(|match_| {
            match_.captures.iter().map(|capture| {
                let range = capture.node.range();
                let occurrence = SymbolOccurrence {
                    start_line: range.start_point.row + 1,
                    start_column: range.start_point.column + 1,
                    end_line: range.end_point.row + 1,
                    end_column: range.end_point.column + 1,
                };
                let matched_text = capture
                    .node
                    .utf8_text(source_code.as_bytes())
                    .unwrap_or("Unable to get text");
                debug!(
                    "Found occurrence: {:?}, Text: '{}'",
                    occurrence, matched_text
                );
                occurrence
            })
        })
        .collect();

    debug!("Total occurrences found: {}", occurrences.len());
    Ok(occurrences)
}
