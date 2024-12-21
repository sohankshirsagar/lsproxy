use super::*;

#[tokio::test]
async fn test_php_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&php_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "AStar.php";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let mut symbol_response: SymbolResponse =
        file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

    let mut expected = vec![
        Symbol {
            name: String::from("AStar"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 4,
                    character: 7,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 4,
                    character: 0,
                },
                end: Position {
                    line: 92,
                    character: 1,
                },
            },
        },
        Symbol {
            name: String::from("findPathTo"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 18,
                    character: 19,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 18,
                    character: 4,
                },
                end: Position {
                    line: 43,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("addNeighborsToOpenList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 45,
                    character: 21,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 45,
                    character: 4,
                },
                end: Position {
                    line: 75,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("distance"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 77,
                    character: 21,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 77,
                    character: 4,
                },
                end: Position {
                    line: 80,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("findNeighborInList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 82,
                    character: 21,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 82,
                    character: 4,
                },
                end: Position {
                    line: 90,
                    character: 5,
                },
            },
        },
    ];

    // sort symbols by name
    symbol_response.sort_by_key(|s| s.name.clone());
    expected.sort_by_key(|s| s.name.clone());
    assert_eq!(symbol_response, expected);
    Ok(())
}

#[tokio::test]
async fn test_php_references() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&php_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "AStar.php";
    let references = manager
        .find_references(
            file_path,
            lsp_types::Position {
                line: 4,
                character: 7,
            },
        )
        .await?;

    let expected = vec![
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/AStar.php").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 4,
                    character: 7,
                },
                end: lsp_types::Position {
                    line: 4,
                    character: 12,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/main.php").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 20,
                    character: 13,
                },
                end: lsp_types::Position {
                    line: 20,
                    character: 18,
                },
            },
        },
    ];
    assert_eq!(references, expected);
    Ok(())
}

#[tokio::test]
async fn test_php_definition() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&php_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let definition_response = manager
        .find_definition(
            "main.php",
            lsp_types::Position {
                line: 20,
                character: 13,
            },
        )
        .await?;

    let definitions = match definition_response {
        GotoDefinitionResponse::Scalar(location) => vec![location],
        GotoDefinitionResponse::Array(locations) => locations,
        GotoDefinitionResponse::Link(_links) => Vec::new(),
    };
    let expected = vec![Location {
        uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/AStar.php").unwrap(),
        range: Range {
            start: lsp_types::Position {
                line: 4,
                character: 7,
            },
            end: lsp_types::Position {
                line: 4,
                character: 12,
            },
        },
    }];

    assert_eq!(definitions, expected);
    Ok(())
}
