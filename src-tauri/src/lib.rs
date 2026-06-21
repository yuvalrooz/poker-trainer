mod poker;

use poker::{apply_action, calculate_equity, new_game, to_view, EquityResult, GameState, GameView};
use std::sync::Mutex;
use tauri::State;

struct AppState(Mutex<Option<GameState>>);

#[tauri::command]
fn new_hand(state: State<AppState>) -> GameView {
    let game = new_game();
    let view = to_view(&game);
    *state.0.lock().unwrap() = Some(game);
    view
}

#[tauri::command]
fn take_action(
    action: String,
    amount: Option<u32>,
    state: State<AppState>,
) -> Result<GameView, String> {
    let mut lock = state.0.lock().unwrap();
    let game = lock.as_mut().ok_or("No active game")?;
    apply_action(game, &action, amount);
    Ok(to_view(game))
}

#[tauri::command]
fn get_equity(state: State<AppState>) -> Result<EquityResult, String> {
    let lock = state.0.lock().unwrap();
    let game = lock.as_ref().ok_or("No active game")?;
    Ok(calculate_equity(game))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState(Mutex::new(None)))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![new_hand, take_action, get_equity])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
