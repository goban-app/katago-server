use crate::config::RequestConfig;
use crate::katago_bot::{Diagnostics, KatagoBot};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

pub type AppState = Arc<KatagoBot>;

#[derive(Debug, Deserialize)]
pub struct SelectMoveRequest {
    #[allow(dead_code)]
    pub board_size: u8,
    pub moves: Vec<String>,
    #[serde(default)]
    pub config: RequestConfig,
}

#[derive(Debug, Serialize)]
pub struct SelectMoveResponse {
    pub bot_move: String,
    pub diagnostics: DiagnosticsResponse,
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ScoreRequest {
    #[allow(dead_code)]
    pub board_size: u8,
    pub moves: Vec<String>,
    #[serde(default)]
    pub config: RequestConfig,
}

#[derive(Debug, Serialize)]
pub struct ScoreResponse {
    pub probs: Vec<f32>,
    pub diagnostics: DiagnosticsResponse,
    pub request_id: String,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticsResponse {
    pub winprob: f32,
    pub score: f32,
    pub bot_move: String,
    pub best_ten: Vec<MoveCandidateResponse>,
}

#[derive(Debug, Serialize)]
pub struct MoveCandidateResponse {
    #[serde(rename = "move")]
    pub mv: String,
    pub psv: i32,
}

impl From<Diagnostics> for DiagnosticsResponse {
    fn from(diag: Diagnostics) -> Self {
        Self {
            winprob: diag.winprob,
            score: diag.score,
            bot_move: diag.bot_move,
            best_ten: diag
                .best_ten
                .into_iter()
                .map(|c| MoveCandidateResponse {
                    mv: c.mv,
                    psv: c.psv,
                })
                .collect(),
        }
    }
}

// Custom error type for API responses
pub struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        error!("API error: {}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": self.0.to_string()
            })),
        )
            .into_response()
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

pub fn create_router(bot: Arc<KatagoBot>) -> Router {
    Router::new()
        .route("/select-move/katago_gtp_bot", post(select_move))
        .route("/score/katago_gtp_bot", post(score))
        .route("/health", axum::routing::get(health))
        .with_state(bot)
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok"
    }))
}

#[axum::debug_handler]
async fn select_move(
    State(bot): State<AppState>,
    Json(request): Json<SelectMoveRequest>,
) -> std::result::Result<Json<SelectMoveResponse>, ApiError> {
    let bot_move = bot.select_move(&request.moves, &request.config).await?;
    let diagnostics = bot.diagnostics();
    let request_id = request.config.request_id.clone().unwrap_or_default();

    Ok(Json(SelectMoveResponse {
        bot_move,
        diagnostics: diagnostics.into(),
        request_id,
    }))
}

#[axum::debug_handler]
async fn score(
    State(bot): State<AppState>,
    Json(request): Json<ScoreRequest>,
) -> std::result::Result<Json<ScoreResponse>, ApiError> {
    let probs = bot.score(&request.moves, &request.config).await?;
    let diagnostics = bot.diagnostics();
    let request_id = request.config.request_id.clone().unwrap_or_default();

    Ok(Json(ScoreResponse {
        probs,
        diagnostics: diagnostics.into(),
        request_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_response_conversion() {
        let diag = Diagnostics {
            winprob: 0.65,
            score: 12.5,
            bot_move: "D4".to_string(),
            best_ten: vec![
                crate::katago_bot::MoveCandidate {
                    mv: "D4".to_string(),
                    psv: 1000,
                },
                crate::katago_bot::MoveCandidate {
                    mv: "Q16".to_string(),
                    psv: 950,
                },
            ],
        };

        let response: DiagnosticsResponse = diag.into();
        assert_eq!(response.winprob, 0.65);
        assert_eq!(response.score, 12.5);
        assert_eq!(response.bot_move, "D4");
        assert_eq!(response.best_ten.len(), 2);
        assert_eq!(response.best_ten[0].mv, "D4");
        assert_eq!(response.best_ten[0].psv, 1000);
        assert_eq!(response.best_ten[1].mv, "Q16");
        assert_eq!(response.best_ten[1].psv, 950);
    }

    #[test]
    fn test_select_move_request_deserialization() {
        let json = r#"{
            "board_size": 19,
            "moves": ["D4", "Q16"],
            "config": {
                "komi": 7.5,
                "client": "test"
            }
        }"#;
        let request: SelectMoveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.board_size, 19);
        assert_eq!(request.moves, vec!["D4", "Q16"]);
        assert_eq!(request.config.komi, Some(7.5));
        assert_eq!(request.config.client, Some("test".to_string()));
    }

    #[test]
    fn test_select_move_request_with_defaults() {
        let json = r#"{
            "board_size": 19,
            "moves": []
        }"#;
        let request: SelectMoveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.board_size, 19);
        assert!(request.moves.is_empty());
        assert!(request.config.komi.is_none());
    }

    #[test]
    fn test_score_request_deserialization() {
        let json = r#"{
            "board_size": 19,
            "moves": ["D4", "Q16", "D16"],
            "config": {
                "ownership": true,
                "request_id": "test-123"
            }
        }"#;
        let request: ScoreRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.board_size, 19);
        assert_eq!(request.moves.len(), 3);
        assert_eq!(request.config.ownership, Some(true));
        assert_eq!(request.config.request_id, Some("test-123".to_string()));
    }

    #[test]
    fn test_select_move_response_serialization() {
        let response = SelectMoveResponse {
            bot_move: "D4".to_string(),
            diagnostics: DiagnosticsResponse {
                winprob: 0.52,
                score: 0.5,
                bot_move: "D4".to_string(),
                best_ten: vec![MoveCandidateResponse {
                    mv: "D4".to_string(),
                    psv: 1000,
                }],
            },
            request_id: "test-id".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"bot_move\":\"D4\""));
        assert!(json.contains("\"winprob\":0.52"));
        assert!(json.contains("\"request_id\":\"test-id\""));
    }

    #[test]
    fn test_score_response_serialization() {
        let response = ScoreResponse {
            probs: vec![0.1, 0.2, 0.7],
            diagnostics: DiagnosticsResponse {
                winprob: 0.75,
                score: 15.0,
                bot_move: "Q16".to_string(),
                best_ten: vec![],
            },
            request_id: "score-test".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"probs\":[0.1,0.2,0.7]"));
        assert!(json.contains("\"winprob\":0.75"));
        assert!(json.contains("\"score\":15.0"));
    }

    #[test]
    fn test_move_candidate_serialization() {
        let candidate = MoveCandidateResponse {
            mv: "D4".to_string(),
            psv: 1234,
        };

        let json = serde_json::to_string(&candidate).unwrap();
        // Note: 'mv' field should be serialized as 'move' due to rename attribute
        assert!(json.contains("\"move\":\"D4\""));
        assert!(json.contains("\"psv\":1234"));
    }
}
