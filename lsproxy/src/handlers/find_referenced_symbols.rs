use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{Position as LspPosition, GotoDefinitionResponse};
use crate::api_types::{Identifier, ErrorResponse, GetReferencedSymbolsRequest, ReferencedSymbolsResponse, Position, FilePosition};
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


    HttpResponse::Ok().json(unwrapped_definition_responses)

}
