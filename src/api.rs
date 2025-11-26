use crate::katago_bot::{Diagnostics, KatagoBot, MoveCandidate};
use crate::config::RequestConfig;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

pub type AppState = Arc<KatagoBot>;

#[derive(Debug, Deserialize)]
pub struct SelectMoveRequest {
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
            best_ten: diag.best_ten.into_iter().map(|c| MoveCandidateResponse {
                mv: c.mv,
                psv: c.psv,
            }).collect(),
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

async fn select_move(
    State(bot): State<AppState>,
    Json(request): Json<SelectMoveRequest>,
) -> Result<Json<SelectMoveResponse>, ApiError> {
    let bot_move = bot.select_move(&request.moves, &request.config).await?;
    let diagnostics = bot.diagnostics();
    let request_id = request.config.request_id.clone().unwrap_or_default();

    Ok(Json(SelectMoveResponse {
        bot_move,
        diagnostics: diagnostics.into(),
        request_id,
    }))
}

async fn score(
    State(bot): State<AppState>,
    Json(request): Json<ScoreRequest>,
) -> Result<Json<ScoreResponse>, ApiError> {
    let probs = bot.score(&request.moves, &request.config).await?;
    let diagnostics = bot.diagnostics();
    let request_id = request.config.request_id.clone().unwrap_or_default();

    Ok(Json(ScoreResponse {
        probs,
        diagnostics: diagnostics.into(),
        request_id,
    }))
}
