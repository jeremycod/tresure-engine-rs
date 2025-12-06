use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;

/// Core game state returned to the client / gateway
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GameState {
    pub game_id: String,
    pub player: PlayerState,
    pub log: Vec<String>,
    pub game_over: bool,
    pub victory: bool,
}

/// Player position & stats
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlayerState {
    pub x: i32,
    pub y: i32,
    pub health: i32,
    pub inventory: Vec<String>,
}

/// Actions that can be requested by the gateway / LLM
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum GameAction {
    #[serde(rename = "move")]
    Move { direction: Direction },
    #[serde(rename = "inspect")]
    Inspect,
    #[serde(rename = "pickup")]
    Pickup,
    #[serde(rename = "use_item")]
    UseItem { item: String },
    #[serde(rename = "attack")]
    Attack,
}

/// Directions for movement
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    North,
    South,
    East,
    West,
}

/// Create a brand new game
pub fn new_game() -> GameState {
    GameState {
        game_id: Uuid::new_v4().to_string(),
        player: PlayerState {
            x: 0,
            y: 0,
            health: 10,
            inventory: vec![],
        },
        log: vec![
            "You wake up in a small village at (0,0). To the east lies a dark forest.".to_string(),
        ],
        game_over: false,
        victory: false,
    }
}

/// Apply an action to the state and return an updated copy
pub fn apply_action(state: &GameState, action: &GameAction) -> GameState {
    // Clone so we keep the original immutable
    let mut new_state = state.clone();

    if new_state.game_over {
        new_state
            .log
            .push("The game is already over. Start a new one to continue playing.".to_string());
        return new_state;
    }

    match action {
        GameAction::Move { direction } => handle_move(&mut new_state, direction),
        GameAction::Inspect => handle_inspect(&mut new_state),
        GameAction::Pickup => handle_pickup(&mut new_state),
        GameAction::UseItem { item } => handle_use_item(&mut new_state, item),
        GameAction::Attack => handle_attack(&mut new_state),
    }

    new_state
}

fn handle_move(state: &mut GameState, direction: &Direction) {
    let (dx, dy) = match direction {
        Direction::North => (0, -1),
        Direction::South => (0, 1),
        Direction::East => (1, 0),
        Direction::West => (-1, 0),
    };

    let new_x = state.player.x + dx;
    let new_y = state.player.y + dy;

    // World bounds: 0..=2 for x, 0..=1 for y
    if new_x < 0 || new_x > 2 || new_y < 0 || new_y > 1 {
        state
            .log
            .push("You can't go that way. The world seems to end there.".to_string());
        return;
    }

    state.player.x = new_x;
    state.player.y = new_y;

    let desc = describe_tile(new_x, new_y);
    state
        .log
        .push(format!("You move to ({},{}) - {}", new_x, new_y, desc));
}

fn handle_inspect(state: &mut GameState) {
    let desc = describe_tile(state.player.x, state.player.y);
    state
        .log
        .push(format!("You inspect your surroundings: {}", desc));
}

fn handle_pickup(state: &mut GameState) {
    let (x, y) = (state.player.x, state.player.y);

    // Very simple item logic:
    // - Forest at (1,0) has a "potion"
    // - Cave entrance at (2,0) has a "rusty key"
    match (x, y) {
        (1, 0) => {
            if !state.player.inventory.contains(&"potion".to_string()) {
                state.player.inventory.push("potion".to_string());
                state
                    .log
                    .push("You find a small potion on the ground and pick it up.".to_string());
            } else {
                state
                    .log
                    .push("You already picked up the potion here.".to_string());
            }
        }
        (2, 0) => {
            if !state.player.inventory.contains(&"rusty key".to_string()) {
                state.player.inventory.push("rusty key".to_string());
                state.log.push(
                    "You notice a rusty key wedged between rocks and carefully take it."
                        .to_string(),
                );
            } else {
                state
                    .log
                    .push("You already picked up the key here.".to_string());
            }
        }
        _ => {
            state
                .log
                .push("You search around but don't find anything interesting.".to_string());
        }
    }
}

fn handle_use_item(state: &mut GameState, item: &str) {
    if !state.player.inventory.contains(&item.to_string()) {
        state
            .log
            .push(format!("You don't have a {} to use.", item));
        return;
    }

    match item {
        "potion" => {
            state.player.health = 10;
            state
                .player
                .inventory
                .retain(|i| i != "potion"); // consume potion
            state
                .log
                .push("You drink the potion. Your health is fully restored.".to_string());
        }
        "rusty key" => {
            if state.player.x == 2 && state.player.y == 1 {
                state.log.push(
                    "You use the rusty key to open the ancient chest in the cave.".to_string(),
                );
                state.log.push(
                    "Inside, you find a pile of gold and a glowing gem. You have found the treasure!"
                        .to_string(),
                );
                state.game_over = true;
                state.victory = true;
            } else {
                state.log.push(
                    "You idly play with the rusty key, but it doesn't seem to fit anything here."
                        .to_string(),
                );
            }
        }
        _ => {
            state
                .log
                .push(format!("You can't figure out how to use the {}.", item));
        }
    }
}

fn handle_attack(state: &mut GameState) {
    // Only meaningful in the deep cave at (2,1)
    let (x, y) = (state.player.x, state.player.y);
    if (x, y) != (2, 1) {
        state
            .log
            .push("You swing at the air. There's nothing to attack here.".to_string());
        return;
    }

    // Simple "combat": 50/50 chance to win or take damage
    let roll = fastrand::u8(0..=100);
    if roll < 50 {
        state
            .log
            .push("You lunge forward and strike the lurking shadow. It vanishes!".to_string());
        state.log.push(
            "With the guardian defeated, you can now safely search for treasure here."
                .to_string(),
        );
    } else {
        let damage = 3;
        state.player.health -= damage;
        state.log.push(format!(
            "A dark creature lashes out from the shadows and hits you for {} damage!",
            damage
        ));

        if state.player.health <= 0 {
            state.log.push("You collapse to the ground. The darkness closes in...".to_string());
            state.game_over = true;
            state.victory = false;
        } else {
            state.log.push("You barely survive the attack and stagger back.".to_string());
        }
    }
}

/// Describe the tile at coordinates (x,y)
fn describe_tile(x: i32, y: i32) -> &'static str {
    match (x, y) {
        (0, 0) => "a quiet village square with a well in the center.",
        (1, 0) => "a dense forest. You hear distant howls and see something glinting on the ground.",
        (2, 0) => "the entrance to a dark cave. Cold air flows from within.",
        (2, 1) => "a deep cave chamber. You feel an ominous presence and see a locked chest.",
        (0, 1) => "a small riverbank. The water is clear and cold.",
        (1, 1) => "a rocky path leading between the forest and the cave.",
        _ => "featureless terrain.",
    }
}
