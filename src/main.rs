mod game;

use crate::game::{apply_action, new_game, GameAction, GameState};
use tracing::{info, error};
use rmcp::model::{CallToolResult, Content, ErrorData, ServerCapabilities, ServerInfo};
use rmcp::{
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    schemars,
    tool,
    tool_handler,
    tool_router,
    transport::stdio,
    ServerHandler,
    ServiceExt,
};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::FmtSubscriber;

// Alias for convenience
type McpError = ErrorData;


/// Parameters for `game_get_state`
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetStateParams {
    pub game_id: String,
}

/// Parameters for `game_apply_action`
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ApplyActionParams {
    pub game_id: String,
    pub action: GameAction,
}

#[derive(Clone)]
pub struct TreasureEngine {
    games: Arc<Mutex<HashMap<String, GameState>>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl TreasureEngine {
    pub fn new() -> Self {
        Self {
            games: Arc::new(Mutex::new(HashMap::new())),
            tool_router: Self::tool_router(),
        }
    }

    /// Start a new game and return the initial GameState
    #[tool(description = "Start a new Treasure Quest game and return the initial state")]
    async fn game_start(&self) -> Result<CallToolResult, McpError> {
        info!("Tool call: game_start - Input: (no parameters)");
        let game = new_game();
        let id = game.game_id.clone();

        let mut games = self.games.lock().await;
        games.insert(id, game.clone());

        let content =
            Content::json(&game).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        
        info!("Tool call: game_start - Output: game_id={}", game.game_id);
        Ok(CallToolResult::success(vec![content]))
    }

    /// Get the current state for a given game_id
    #[tool(description = "Get the current game state by game_id")]
    async fn game_get_state(
        &self,
        params: Parameters<GetStateParams>,
    ) -> Result<CallToolResult, McpError> {
        let GetStateParams { game_id } = params.0;
        info!("Tool call: game_get_state - Input: game_id={}", game_id);

        let games = self.games.lock().await;
        let state = games
            .get(&game_id)
            .cloned()
            .ok_or_else(|| {
                error!("Tool call: game_get_state - Error: No game found for id {}", game_id);
                McpError::invalid_params(
                    format!("No game found for id {}", game_id),
                    None, // data
                )
            })?;

        let content =
            Content::json(&state).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        
        info!("Tool call: game_get_state - Output: returned state for game_id={}", game_id);
        Ok(CallToolResult::success(vec![content]))
    }
    /// Apply an action to the game and return the updated state
    #[tool(description = "Apply an action to the game and return updated state")]
    async fn game_apply_action(
        &self,
        params: Parameters<ApplyActionParams>,
    ) -> Result<CallToolResult, McpError> {
        let ApplyActionParams { game_id, action } = params.0;
        info!("Tool call: game_apply_action - Input: game_id={}, action={:?}", game_id, action);

        let mut games = self.games.lock().await;
        let current = games
            .get(&game_id)
            .cloned()
            .ok_or_else(|| {
                error!("Tool call: game_apply_action - Error: No game found for id {}", game_id);
                McpError::invalid_params(
                    format!("No game found for id {}", game_id),
                    None, // data
                )
            })?;

        let updated = apply_action(&current, &action);
        let game_id_clone = game_id.clone();
        games.insert(game_id, updated.clone());

        let content =
            Content::json(&updated).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        
        info!("Tool call: game_apply_action - Output: updated state for game_id={}", game_id_clone);
        Ok(CallToolResult::success(vec![content]))
    }
}

#[tool_handler]
impl ServerHandler for TreasureEngine {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "A simple Treasure Quest game engine. Start a game with `game_start`, \
                 then use `game_apply_action` with actions like move/inspect/pickup/use_item/attack."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr) // important: logs to stderr, not stdout
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
    // Run the server over stdio (works with your TS gateway)
    let service = TreasureEngine::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            eprintln!("Error starting TreasureEngine server: {e}");
        })?;

    service.waiting().await?;
    Ok(())
}
