use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{Position as LspPosition, GotoDefinitionResponse};
use crate::api_types::{ErrorResponse, FileRange, GetReferencedSymbolsRequest, ReferencedSymbolsResponse, Symbol, Position, FilePosition};
use crate::{ast_grep, AppState};
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

    let referenecd_ast_symbols = manager
        .find_referenced_symbols(
            &info.identifier_position.path,
            LspPosition {
                line: info.identifier_position.position.line,
                character: info.identifier_position.position.character,
            },
        )
        .await
        .map_err(|e| {
            error!("Referenced symbols error: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Finding referenced symbols failed: {}", e),
            })
        })
        .unwrap();
    let referenced_symbols = referenecd_ast_symbols.into_iter().map(|(ast_grep_result, definition_response)| async {
        let definitions = match definition_response {
            GotoDefinitionResponse::Scalar(location) => vec![(uri_to_relative_path_string(&location.uri), LspPosition { line: location.range.start.line, character: location.range.start.character })],
            GotoDefinitionResponse::Array(locations) => {
                locations.into_iter().map(|location| (uri_to_relative_path_string(&location.uri), LspPosition { line: location.range.start.line, character: location.range.start.character })).collect()
            }
            GotoDefinitionResponse::Link(links) => {
                links.into_iter().map(|link| (uri_to_relative_path_string(&link.target_uri), LspPosition { line: link.target_range.start.line, character: link.target_range.start.character })).collect()
            }
        };
        match definitions.first() {
            Some(def_item) => Some(manager.get_symbol_from_position(&def_item.0, &def_item.1).await
            .map_err(|e| Symbol {name: ast_grep_result.meta_variables.single.name.text.clone(), kind: ast_grep_result.rule_id.clone(), identifier_position: FilePosition { path: String::from("NOTFOUND"), position: Position {line: 0, character: 0}}, range: FileRange {path: String::from("NOTFOUND"), start: Position {line: 0, character: 0}, end: Position {line: 0, character: 0}}})
        ),
            None => None
        }

    }).collect();

    HttpResponse::Ok().json("")

}
