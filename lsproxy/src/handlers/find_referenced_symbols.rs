use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{Position as LspPosition, GotoDefinitionResponse};
use crate::api_types::{Identifier, ErrorResponse, GetReferencedSymbolsRequest, ReferencedSymbolsResponse, Position, FilePosition, SymbolWithIdentifier};
use crate::AppState;
use crate::utils::file_utils::uri_to_relative_path_string;

#[utoipa::path(
    post,
    path = "/symbol/find-referenced-symbols",
    tag = "symbol",
    request_body = GetReferencedSymbolsRequest,
    responses(
        (status = 200, description = "Referenced symbols retrieved successfully", body = ReferencedSymbolsResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_referenced_symbols(
    data: Data<AppState>,
    info: Json<GetReferencedSymbolsRequest>,
) -> HttpResponse {
    info!(
        "Received referenced symbols request for file: {}, line: {}, character: {}",
        info.identifier_position.path,
        info.identifier_position.position.line,
        info.identifier_position.position.character
    );

    let manager = data.manager.lock().unwrap();

    let referenecd_ast_symbols = match manager
        .find_referenced_symbols(
            &info.identifier_position.path,
            LspPosition {
                line: info.identifier_position.position.line,
                character: info.identifier_position.position.character,
            },
        )
        .await {
            Ok(ast_symbols) => ast_symbols,
            Err(e) => {
                error!("Failed to get referenced symbols: {:?}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse {
                    error: format!("Failed to get referenced symbols: {}", e),
                });
            }
        };

    let unwrapped_definition_responses: Vec<(Identifier, Vec<FilePosition>)> = referenecd_ast_symbols.into_iter().map(|(ast_grep_result, definition_response)| {
        let definitions = match definition_response {
            GotoDefinitionResponse::Scalar(location) => vec![FilePosition { path: uri_to_relative_path_string(&location.uri), position: Position { line: location.range.start.line, character: location.range.start.character }}],
            GotoDefinitionResponse::Array(locations) => {
                locations.into_iter().map(|location| FilePosition { path: uri_to_relative_path_string(&location.uri), position: Position { line: location.range.start.line, character: location.range.start.character }}).collect()
            }
            GotoDefinitionResponse::Link(links) => {
                links.into_iter().map(|link| FilePosition { path: uri_to_relative_path_string(&link.target_uri), position: Position { line: link.target_range.start.line, character: link.target_range.start.character }}).collect()
            }
        };
        (Identifier::from(ast_grep_result), definitions)
    }).collect();


    // First get the workspace files
    let files = match manager.list_files().await {
        Ok(files) => files,
        Err(e) => {
            error!("Failed to list workspace files: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to list workspace files: {}", e),
            });
        }
    };

    // Then categorize the definitions
    let mut workspace_symbols = Vec::new();
    let mut builtin_symbols = Vec::new();
    let mut not_found = Vec::new();

    for (identifier, definitions) in unwrapped_definition_responses {
        if definitions.is_empty() {
            not_found.push(identifier);
        } else {
            // Check if any definition is in workspace files
            let has_internal_definition = definitions.iter().any(|def| files.contains(&def.path));
            
            if has_internal_definition {
                let mut symbols_with_definitions = Vec::new();
                for def in definitions.iter().filter(|def| files.contains(&def.path)) {
                    if let Ok(symbol) = manager
                        .get_symbol_from_position(
                            &def.path,
                            &lsp_types::Position {
                                line: def.position.line,
                                character: def.position.character,
                            },
                        )
                        .await
                    {
                        symbols_with_definitions.push(symbol);
                    }
                }

                workspace_symbols.push(SymbolWithIdentifier {
                    identifier: identifier.clone(),
                    definitions: symbols_with_definitions,
                });
            } else {
                builtin_symbols.push(identifier.clone());
            }
        }
    }

    // Return the categorized response
    HttpResponse::Ok().json(ReferencedSymbolsResponse {
        workspace_symbols,
        builtin_symbols,
        not_found,
    })

}
