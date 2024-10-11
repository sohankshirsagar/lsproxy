use tree_sitter::{Parser, Query, QueryCursor};
use std::fs;

pub struct SymbolOccurrence {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

pub fn find_symbol_occurrences(file_path: &str, symbol_name: &str) -> Result<Vec<SymbolOccurrence>, Box<dyn std::error::Error>> {
    // Read the file content
    let source_code = fs::read_to_string(file_path)?;

    // Initialize the parser
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_python::language())?;

    // Parse the source code
    let tree = parser.parse(&source_code, None).ok_or("Failed to parse the source code")?;

    // Create a query to find the symbol
    let query = Query::new(tree_sitter_python::language(), &format!(r#"
        (identifier) @id
        (#eq? @id "{}")
    "#, symbol_name))?;

    let mut query_cursor = QueryCursor::new();
    let matches = query_cursor.matches(&query, tree.root_node(), source_code.as_bytes());

    let mut occurrences = Vec::new();

    for match_ in matches {
        for capture in match_.captures {
            let range = capture.node.range();
            occurrences.push(SymbolOccurrence {
                start_line: range.start_point.row + 1,
                start_column: range.start_point.column + 1,
                end_line: range.end_point.row + 1,
                end_column: range.end_point.column + 1,
            });
        }
    }

    Ok(occurrences)
}
