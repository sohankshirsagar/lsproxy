use super::*;

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&csharp_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let file_path = "AStar.cs";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let symbol_response: SymbolResponse =
        file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

    let expected = vec![
        Symbol {
            name: String::from("AStar"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 2,
                    character: 17,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 2,
                    character: 0,
                },
                end: Position {
                    line: 87,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("_open"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 4,
                    character: 36,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 4,
                    character: 0,
                },
                end: Position {
                    line: 4,
                    character: 50,
                },
            },
        },
        Symbol {
            name: String::from("_closed"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 5,
                    character: 36,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 5,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 52,
                },
            },
        },
        Symbol {
            name: String::from("_path"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 6,
                    character: 36,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 6,
                    character: 0,
                },
                end: Position {
                    line: 6,
                    character: 50,
                },
            },
        },
        Symbol {
            name: String::from("_maze"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 7,
                    character: 33,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 7,
                    character: 0,
                },
                end: Position {
                    line: 7,
                    character: 39,
                },
            },
        },
        Symbol {
            name: String::from("_current"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 8,
                    character: 21,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 8,
                    character: 0,
                },
                end: Position {
                    line: 8,
                    character: 30,
                },
            },
        },
        Symbol {
            name: String::from("_xStart"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 9,
                    character: 29,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 9,
                    character: 0,
                },
                end: Position {
                    line: 9,
                    character: 37,
                },
            },
        },
        Symbol {
            name: String::from("_yStart"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 10,
                    character: 29,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 10,
                    character: 37,
                },
            },
        },
        Symbol {
            name: String::from("_xEnd"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 11,
                    character: 20,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 11,
                    character: 0,
                },
                end: Position {
                    line: 11,
                    character: 33,
                },
            },
        },
        Symbol {
            name: String::from("_yEnd"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 11,
                    character: 27,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 11,
                    character: 0,
                },
                end: Position {
                    line: 11,
                    character: 33,
                },
            },
        },
        Symbol {
            name: String::from("_diag"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 12,
                    character: 30,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 12,
                    character: 0,
                },
                end: Position {
                    line: 12,
                    character: 36,
                },
            },
        },
        Symbol {
            name: String::from("FindPathTo"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 23,
                    character: 27,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 23,
                    character: 0,
                },
                end: Position {
                    line: 49,
                    character: 9,
                },
            },
        },
        Symbol {
            name: String::from("AddNeighborsToOpenList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 51,
                    character: 21,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 51,
                    character: 0,
                },
                end: Position {
                    line: 76,
                    character: 9,
                },
            },
        },
        Symbol {
            name: String::from("Distance"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 78,
                    character: 23,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 78,
                    character: 0,
                },
                end: Position {
                    line: 81,
                    character: 9,
                },
            },
        },
        Symbol {
            name: String::from("FindNeighborInList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 83,
                    character: 21,
                },
            },
            range: FileRange {
                path: String::from("AStar.cs"),
                start: Position {
                    line: 83,
                    character: 0,
                },
                end: Position {
                    line: 86,
                    character: 9,
                },
            },
        },
    ];
    assert_eq!(symbol_response, expected);
    Ok(())
}
