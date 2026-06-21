use rand::seq::SliceRandom;
use rand::{Rng, RngExt};
use rs_poker::core::{Card as RsCard, Rankable, Suit as RsSuit, Value};
use serde::{Deserialize, Serialize};

// ── Domain types ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Card {
    pub rank: u8, // 2–14
    pub suit: Suit,
}

impl Card {
    pub fn new(rank: u8, suit: Suit) -> Self {
        Card { rank, suit }
    }
    fn to_rs(&self) -> RsCard {
        let v = match self.rank {
            2 => Value::Two,   3 => Value::Three, 4 => Value::Four,
            5 => Value::Five,  6 => Value::Six,   7 => Value::Seven,
            8 => Value::Eight, 9 => Value::Nine,  10 => Value::Ten,
            11 => Value::Jack, 12 => Value::Queen, 13 => Value::King,
            _ => Value::Ace,
        };
        let s = match self.suit {
            Suit::Clubs => RsSuit::Club, Suit::Diamonds => RsSuit::Diamond,
            Suit::Hearts => RsSuit::Heart, Suit::Spades => RsSuit::Spade,
        };
        RsCard::new(v, s)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
}

impl std::fmt::Display for Street {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Street::Preflop => "Preflop", Street::Flop => "Flop",
            Street::Turn => "Turn", Street::River => "River", Street::Showdown => "Showdown",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PlayerStatus {
    Active,
    Folded,
    AllIn,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub id: usize,
    pub name: String,
    pub position: String,
    pub stack: u32,
    pub hole_cards: Option<[Card; 2]>,
    pub is_hero: bool,
    pub status: PlayerStatus,
    pub current_bet: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActionRecord {
    pub player_name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LegalActions {
    pub can_fold: bool,
    pub can_check: bool,
    pub can_call: bool,
    pub call_amount: u32,
    pub can_bet: bool,
    pub min_bet: u32,
    pub can_raise: bool,
    pub min_raise: u32,
    pub max_amount: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EquityResult {
    pub win: f64,
    pub tie: f64,
    pub lose: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct SampleTrial {
    pub extra_community: Vec<Card>,
    pub opponent_cards: Vec<Card>,
    pub hero_rank: String,
    pub best_opp_rank: String,
    pub result: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct EquityDetail {
    pub win: f64,
    pub tie: f64,
    pub lose: f64,
    pub trials: usize,
    pub remaining_deck_size: usize,
    pub cards_to_come: usize,
    pub cards_per_opp: usize,
    pub num_opponents: usize,
    pub sample_trials: Vec<SampleTrial>,
}

#[derive(Clone, Debug)]
pub struct DecisionSnapshot {
    pub street: String,
    pub pot: u32,
    pub call_amount: u32,
    pub action_taken: String,
    pub community_cards: Vec<Card>,
    pub hero_hole_cards: [Card; 2],
    pub active_opponents: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecisionVerdict {
    pub street: String,
    pub action: String,
    pub pot_odds_pct: f64,
    pub equity_pct: f64,
    pub verdict: String,
    pub correct: bool,
}

// ── Internal game state ───────────────────────────────────────────────────────

pub struct GameState {
    pub players: Vec<Player>,
    pub board: Vec<Card>,           // all 5, revealed incrementally
    pub community_cards: Vec<Card>,
    pub pot: u32,
    pub street: Street,
    pub button_pos: usize,
    pub small_blind: u32,
    pub big_blind: u32,
    pub action_history: Vec<ActionRecord>,
    pub hero_to_act: bool,
    pub street_bet: u32,
    pub action_on: usize,
    pub needs_action: Vec<bool>,
    pub hand_over: bool,
    pub result_message: Option<String>,
    pub legal_actions: LegalActions,
    pub decision_snapshots: Vec<DecisionSnapshot>,
}

// ── Serialisable frontend view ────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameView {
    pub players: Vec<Player>,
    pub community_cards: Vec<Card>,
    pub pot: u32,
    pub street: Street,
    pub action_history: Vec<ActionRecord>,
    pub hero_to_act: bool,
    pub hand_over: bool,
    pub result_message: Option<String>,
    pub legal_actions: LegalActions,
    pub analysis: Option<Vec<DecisionVerdict>>,
}

pub fn to_view(state: &GameState) -> GameView {
    let at_showdown = state.hand_over && state.street == Street::Showdown;
    let players = state.players.iter().map(|p| Player {
        hole_cards: if p.is_hero || at_showdown { p.hole_cards.clone() } else { None },
        ..p.clone()
    }).collect();

    let analysis = if state.hand_over {
        Some(analyze_hand(&state.decision_snapshots))
    } else {
        None
    };

    GameView {
        players,
        community_cards: state.community_cards.clone(),
        pot: state.pot,
        street: state.street.clone(),
        action_history: state.action_history.clone(),
        hero_to_act: state.hero_to_act,
        hand_over: state.hand_over,
        result_message: state.result_message.clone(),
        legal_actions: state.legal_actions.clone(),
        analysis,
    }
}

// ── Deck helpers ──────────────────────────────────────────────────────────────

fn full_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(52);
    for rank in 2u8..=14 {
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            deck.push(Card::new(rank, suit));
        }
    }
    deck
}

pub fn card_str(c: &Card) -> String {
    let r = match c.rank {
        14 => "A", 13 => "K", 12 => "Q", 11 => "J", 10 => "T",
        9 => "9", 8 => "8", 7 => "7", 6 => "6", 5 => "5", 4 => "4", 3 => "3", _ => "2",
    };
    let s = match c.suit {
        Suit::Clubs => "♣", Suit::Diamonds => "♦", Suit::Hearts => "♥", Suit::Spades => "♠",
    };
    format!("{}{}", r, s)
}

fn position_name(offset_from_btn: usize, num_players: usize) -> &'static str {
    // offset 0 = BTN, 1 = SB, 2 = BB, then UTG, HJ, CO etc
    match (offset_from_btn, num_players) {
        (0, _) => "BTN",
        (1, _) => "SB",
        (2, _) => "BB",
        (3, 4) => "UTG",
        (3, 5) => "UTG",
        (3, _) => "UTG",
        (4, 5) => "CO",
        (4, 6) => "HJ",
        (5, 6) => "CO",
        _ => "MP",
    }
}

fn hand_strength(hole: &[Card; 2]) -> u32 {
    hole[0].rank as u32 + hole[1].rank as u32
}

// ── Scenario generator (v0.2.0) ───────────────────────────────────────────────

pub fn new_game() -> GameState {
    let mut rng = rand::rng();

    let num_players: usize = rng.random_range(3..=6);
    let sb = 50u32;
    let bb = 100u32;
    let hero_idx = 0usize;
    let button_pos: usize = rng.random_range(0..num_players);

    // Stacks: 20–150 bb each
    let raw_stacks: Vec<u32> = (0..num_players)
        .map(|_| rng.random_range(20u32..=150) * bb)
        .collect();

    // Shuffle deck and deal
    let mut deck = full_deck();
    deck.shuffle(&mut rng);
    let mut di = 0usize;

    let hole_cards: Vec<[Card; 2]> = (0..num_players).map(|_| {
        let c = [deck[di].clone(), deck[di + 1].clone()];
        di += 2;
        c
    }).collect();

    let board: Vec<Card> = (0..5).map(|_| { let c = deck[di].clone(); di += 1; c }).collect();

    // Decide starting street (weighted)
    let roll: u32 = rng.random_range(0..100);
    let starting_street = if roll < 40 { Street::Preflop }
                          else if roll < 72 { Street::Flop }
                          else if roll < 88 { Street::Turn }
                          else { Street::River };

    build_scenario(
        num_players, hero_idx, button_pos, raw_stacks,
        hole_cards, board, sb, bb, starting_street, &mut rng,
    )
}

fn build_scenario(
    num_players: usize,
    hero_idx: usize,
    button_pos: usize,
    raw_stacks: Vec<u32>,
    hole_cards: Vec<[Card; 2]>,
    board: Vec<Card>,
    sb: u32,
    bb: u32,
    starting_street: Street,
    rng: &mut impl Rng,
) -> GameState {
    // Assign positions
    let mut players: Vec<Player> = (0..num_players).map(|i| {
        let offset = (i + num_players - button_pos) % num_players;
        Player {
            id: i,
            name: if i == hero_idx { "Hero".into() } else { format!("Villain {}", i) },
            position: position_name(offset, num_players).to_string(),
            stack: raw_stacks[i],
            hole_cards: Some(hole_cards[i].clone()),
            is_hero: i == hero_idx,
            status: PlayerStatus::Active,
            current_bet: 0,
        }
    }).collect();

    let sb_idx = (button_pos + 1) % num_players;
    let bb_idx = (button_pos + 2) % num_players;

    // Post blinds
    let sb_post = sb.min(players[sb_idx].stack);
    players[sb_idx].stack -= sb_post;
    players[sb_idx].current_bet = sb_post;
    let bb_post = bb.min(players[bb_idx].stack);
    players[bb_idx].stack -= bb_post;
    players[bb_idx].current_bet = bb_post;
    let mut pot = sb_post + bb_post;

    let mut action_history = vec![
        ActionRecord { player_name: players[sb_idx].name.clone(), description: format!("posts SB {}", sb_post) },
        ActionRecord { player_name: players[bb_idx].name.clone(), description: format!("posts BB {}", bb_post) },
    ];

    let community_cards: Vec<Card>;
    let street: Street;
    let needs_action: Vec<bool>;
    let street_bet: u32;
    let action_on: usize;

    if starting_street == Street::Preflop {
        // Preflop: first actor = (button_pos + 3) % n  (= button_pos for 3-handed)
        community_cards = vec![];
        street = Street::Preflop;
        street_bet = bb;
        needs_action = vec![true; num_players];
        // Preflop first actor
        action_on = if num_players == 3 { button_pos } else { (button_pos + 3) % num_players };
    } else {
        // Simulate preflop to build pot / eliminate players
        simulate_street_bots(&mut players, &mut pot, button_pos, bb, bb, true, rng, &mut action_history);

        // Ensure hero + at least 1 opponent remain; fall back to preflop otherwise
        let hero_active = players[hero_idx].status == PlayerStatus::Active;
        let opp_active = players.iter().filter(|p| !p.is_hero && p.status == PlayerStatus::Active).count();
        if !hero_active || opp_active == 0 {
            // Fallback: restart as preflop hand
            for p in &mut players {
                p.status = PlayerStatus::Active;
                p.stack = raw_stacks[p.id];
                p.current_bet = 0;
            }
            pot = 0;
            action_history.clear();
            let sb_post2 = sb.min(players[sb_idx].stack);
            players[sb_idx].stack -= sb_post2;
            players[sb_idx].current_bet = sb_post2;
            let bb_post2 = bb.min(players[bb_idx].stack);
            players[bb_idx].stack -= bb_post2;
            players[bb_idx].current_bet = bb_post2;
            pot = sb_post2 + bb_post2;
            action_history.push(ActionRecord { player_name: players[sb_idx].name.clone(), description: format!("posts SB {}", sb_post2) });
            action_history.push(ActionRecord { player_name: players[bb_idx].name.clone(), description: format!("posts BB {}", bb_post2) });

            community_cards = vec![];
            street = Street::Preflop;
            street_bet = bb;
            needs_action = vec![true; num_players];
            action_on = if num_players == 3 { button_pos } else { (button_pos + 3) % num_players };
        } else {
            // Reset per-street bets
            for p in &mut players { p.current_bet = 0; }

            // Deal with multi-street starting points
            if matches!(starting_street, Street::Turn | Street::River) {
                let flop_str = format!("--- Flop: {} {} {} ---", card_str(&board[0]), card_str(&board[1]), card_str(&board[2]));
                action_history.push(ActionRecord { player_name: "".into(), description: flop_str });
                simulate_street_bots(&mut players, &mut pot, button_pos, 0, bb, false, rng, &mut action_history);
                for p in &mut players { p.current_bet = 0; }
            }

            if starting_street == Street::River {
                let turn_str = format!("--- Turn: {} ---", card_str(&board[3]));
                action_history.push(ActionRecord { player_name: "".into(), description: turn_str });
                simulate_street_bots(&mut players, &mut pot, button_pos, 0, bb, false, rng, &mut action_history);
                for p in &mut players { p.current_bet = 0; }
            }

            // Set community cards for starting street
            match starting_street {
                Street::Flop => {
                    community_cards = board[0..3].to_vec();
                    street = Street::Flop;
                    let s = format!("--- Flop: {} {} {} ---", card_str(&board[0]), card_str(&board[1]), card_str(&board[2]));
                    action_history.push(ActionRecord { player_name: "".into(), description: s });
                }
                Street::Turn => {
                    community_cards = board[0..4].to_vec();
                    street = Street::Turn;
                    let s = format!("--- Turn: {} ---", card_str(&board[3]));
                    action_history.push(ActionRecord { player_name: "".into(), description: s });
                }
                Street::River => {
                    community_cards = board[0..5].to_vec();
                    street = Street::River;
                    let s = format!("--- River: {} ---", card_str(&board[4]));
                    action_history.push(ActionRecord { player_name: "".into(), description: s });
                }
                _ => unreachable!(),
            }

            street_bet = 0;
            let active: Vec<usize> = (0..num_players).filter(|&i| players[i].status == PlayerStatus::Active).collect();
            needs_action = (0..num_players).map(|i| players[i].status == PlayerStatus::Active).collect();
            action_on = first_post_flop_actor_list(button_pos, &active, num_players);
        }
    }

    // Determine if the first actor is hero
    let hero_to_act = players[action_on].is_hero;
    let legal = if hero_to_act {
        compute_legal(&players, action_on, street_bet, bb)
    } else {
        default_legal()
    };

    let mut state = GameState {
        players,
        board,
        community_cards,
        pot,
        street,
        button_pos,
        small_blind: sb,
        big_blind: bb,
        action_history,
        hero_to_act,
        street_bet,
        action_on,
        needs_action,
        hand_over: false,
        result_message: None,
        legal_actions: legal,
        decision_snapshots: vec![],
    };

    // If first actor is a bot, run it immediately
    if !state.hero_to_act {
        run_bot_chain(&mut state);
    }

    state
}

// Simulate an entire street of bot-only action (used for prior streets).
// hero_to_act=false so hero is skipped; bots check/call/fold.
fn simulate_street_bots(
    players: &mut Vec<Player>,
    pot: &mut u32,
    button_pos: usize,
    street_bet: u32,
    bb: u32,
    is_preflop: bool,
    rng: &mut impl Rng,
    history: &mut Vec<ActionRecord>,
) {
    let n = players.len();
    let mut sb = street_bet;
    let mut needs: Vec<bool> = (0..n).map(|i| players[i].status == PlayerStatus::Active && !players[i].is_hero).collect();

    let first = if is_preflop {
        if n == 3 { button_pos } else { (button_pos + 3) % n }
    } else {
        first_post_flop_actor_list(button_pos, &(0..n).filter(|&i| players[i].status == PlayerStatus::Active).collect::<Vec<_>>(), n)
    };

    // Single pass (no raises from bots, so one round suffices)
    for offset in 0..n {
        let i = (first + offset) % n;
        if !needs[i] || players[i].status != PlayerStatus::Active || players[i].is_hero { continue; }

        let call_amt = sb.saturating_sub(players[i].current_bet);
        let strength = players[i].hole_cards.as_ref().map(hand_strength).unwrap_or(0);

        let folds = if call_amt == 0 {
            false
        } else {
            // Fold probability based on hand strength
            let fold_pct = if strength >= 22 { 5u32 }
                           else if strength >= 18 { 25 }
                           else if strength >= 15 { 55 }
                           else { 75 };
            rng.random_range(0..100) < fold_pct
        };

        if folds {
            players[i].status = PlayerStatus::Folded;
            history.push(ActionRecord { player_name: players[i].name.clone(), description: "folds".into() });
        } else {
            let actual = call_amt.min(players[i].stack);
            players[i].stack -= actual;
            players[i].current_bet += actual;
            *pot += actual;
            let desc = if call_amt == 0 { "checks".into() } else { format!("calls {}", actual) };
            history.push(ActionRecord { player_name: players[i].name.clone(), description: desc });
        }
        needs[i] = false;
        let _ = sb; // sb unchanged (bots don't raise in prior sim)
    }

    // Reset current_bet after the street (caller must do this)
    // (Intentionally left to the caller for clarity)
    let _ = bb;
}

fn first_post_flop_actor_list(button_pos: usize, active: &[usize], n: usize) -> usize {
    for offset in 1..=n {
        let idx = (button_pos + offset) % n;
        if active.contains(&idx) {
            return idx;
        }
    }
    active[0]
}

// ── Action engine ─────────────────────────────────────────────────────────────

pub fn apply_action(state: &mut GameState, action: &str, amount: Option<u32>) {
    let actor = state.action_on;

    // Record decision snapshot before applying hero's action
    if state.players[actor].is_hero {
        let call_amount = state.street_bet.saturating_sub(state.players[actor].current_bet);
        let opp_count = state.players.iter().filter(|p| !p.is_hero && p.status == PlayerStatus::Active).count();
        if let Some(cards) = state.players[actor].hole_cards.clone() {
            state.decision_snapshots.push(DecisionSnapshot {
                street: state.street.to_string(),
                pot: state.pot,
                call_amount,
                action_taken: action.to_string(),
                community_cards: state.community_cards.clone(),
                hero_hole_cards: cards,
                active_opponents: opp_count,
            });
        }
    }

    match action {
        "fold" => {
            push_record(state, actor, "folds");
            state.players[actor].status = PlayerStatus::Folded;
            state.needs_action[actor] = false;
        }
        "check" => {
            push_record(state, actor, "checks");
            state.needs_action[actor] = false;
        }
        "call" => {
            let to_call = state.street_bet.saturating_sub(state.players[actor].current_bet);
            let actual = to_call.min(state.players[actor].stack);
            state.players[actor].stack -= actual;
            state.players[actor].current_bet += actual;
            state.pot += actual;
            push_record(state, actor, &format!("calls {}", actual));
            state.needs_action[actor] = false;
        }
        "bet" => {
            let amt = amount.unwrap_or(state.big_blind);
            let actual = amt.min(state.players[actor].stack);
            state.players[actor].stack -= actual;
            state.players[actor].current_bet += actual;
            state.pot += actual;
            state.street_bet = state.players[actor].current_bet;
            push_record(state, actor, &format!("bets {}", actual));
            state.needs_action[actor] = false;
            reopen_action(state, actor);
        }
        "raise" => {
            let total = amount.unwrap_or(state.street_bet * 2);
            let to_add = total.saturating_sub(state.players[actor].current_bet).min(state.players[actor].stack);
            state.players[actor].stack -= to_add;
            state.players[actor].current_bet += to_add;
            state.pot += to_add;
            state.street_bet = state.players[actor].current_bet;
            push_record(state, actor, &format!("raises to {}", state.street_bet));
            state.needs_action[actor] = false;
            reopen_action(state, actor);
        }
        _ => {}
    }

    advance(state);
}

fn push_record(state: &mut GameState, actor: usize, desc: &str) {
    state.action_history.push(ActionRecord {
        player_name: state.players[actor].name.clone(),
        description: desc.to_string(),
    });
}

fn reopen_action(state: &mut GameState, aggressor: usize) {
    for i in 0..state.players.len() {
        if i != aggressor && state.players[i].status == PlayerStatus::Active {
            state.needs_action[i] = true;
        }
    }
}

fn advance(state: &mut GameState) {
    let n = state.players.len();
    let active: Vec<usize> = (0..n).filter(|&i| state.players[i].status == PlayerStatus::Active).collect();

    if active.len() == 1 {
        let w = active[0];
        state.players[w].stack += state.pot;
        state.result_message = Some(format!("{} wins {} (others folded)", state.players[w].name, state.pot));
        state.hand_over = true;
        state.hero_to_act = false;
        return;
    }

    let round_over = !state.needs_action.iter().enumerate().any(|(i, &needs)| {
        needs && state.players[i].status == PlayerStatus::Active
    });

    if round_over {
        next_street(state);
        return;
    }

    let cur = state.action_on;
    for offset in 1..=n {
        let idx = (cur + offset) % n;
        if state.needs_action[idx] && state.players[idx].status == PlayerStatus::Active {
            state.action_on = idx;
            if state.players[idx].is_hero {
                state.hero_to_act = true;
                state.legal_actions = compute_legal(&state.players, idx, state.street_bet, state.big_blind);
            } else {
                state.hero_to_act = false;
                run_bot(state, idx);
            }
            return;
        }
    }
}

fn next_street(state: &mut GameState) {
    for p in &mut state.players { p.current_bet = 0; }
    state.street_bet = 0;
    for i in 0..state.players.len() {
        state.needs_action[i] = state.players[i].status == PlayerStatus::Active;
    }

    let active: Vec<usize> = (0..state.players.len())
        .filter(|&i| state.players[i].status == PlayerStatus::Active)
        .collect();

    let first = first_post_flop_actor_list(state.button_pos, &active, state.players.len());

    match state.street {
        Street::Preflop => {
            state.street = Street::Flop;
            state.community_cards = state.board[0..3].to_vec();
            let s = format!("--- Flop: {} {} {} ---", card_str(&state.board[0]), card_str(&state.board[1]), card_str(&state.board[2]));
            state.action_history.push(ActionRecord { player_name: "".into(), description: s });
        }
        Street::Flop => {
            state.street = Street::Turn;
            state.community_cards = state.board[0..4].to_vec();
            let s = format!("--- Turn: {} ---", card_str(&state.board[3]));
            state.action_history.push(ActionRecord { player_name: "".into(), description: s });
        }
        Street::Turn => {
            state.street = Street::River;
            state.community_cards = state.board[0..5].to_vec();
            let s = format!("--- River: {} ---", card_str(&state.board[4]));
            state.action_history.push(ActionRecord { player_name: "".into(), description: s });
        }
        Street::River => {
            state.street = Street::Showdown;
            state.community_cards = state.board[0..5].to_vec();
            resolve_showdown(state);
            return;
        }
        Street::Showdown => return,
    }

    state.action_on = first;
    if state.players[first].is_hero {
        state.hero_to_act = true;
        state.legal_actions = compute_legal(&state.players, first, state.street_bet, state.big_blind);
    } else {
        state.hero_to_act = false;
        run_bot_chain(state);
    }
}

// ── Bot logic ─────────────────────────────────────────────────────────────────

fn run_bot(state: &mut GameState, actor: usize) {
    let call_amount = state.street_bet.saturating_sub(state.players[actor].current_bet);
    let strength = state.players[actor].hole_cards.as_ref().map(hand_strength).unwrap_or(0);
    let pot = state.pot;
    let bb = state.big_blind;
    let stack = state.players[actor].stack;

    let (action, amount): (&str, Option<u32>) = if call_amount == 0 {
        // No bet facing — check or bet
        if strength >= 23 {
            // Strong hand: bet ~60% pot
            let bet = ((pot * 6 / 10).max(bb)).min(stack);
            ("bet", Some(bet))
        } else if strength >= 20 && rand::random::<u8>() % 3 == 0 {
            // Medium-strong: bet ~40% pot occasionally
            let bet = ((pot * 4 / 10).max(bb)).min(stack);
            ("bet", Some(bet))
        } else {
            ("check", None)
        }
    } else {
        // Facing a bet — fold, call, or raise
        let fold_threshold = strength.saturating_sub(10) * 25;
        if call_amount > fold_threshold {
            ("fold", None)
        } else if strength >= 25 {
            // Very strong: raise to ~3x the current bet
            let raise_to = (state.street_bet * 3).min(stack);
            ("raise", Some(raise_to))
        } else {
            ("call", None)
        }
    };

    apply_action(state, action, amount);
}

fn run_bot_chain(state: &mut GameState) {
    // Run bots until it's the hero's turn or the hand ends
    while !state.hero_to_act && !state.hand_over {
        let actor = state.action_on;
        if state.players[actor].is_hero {
            break;
        }
        run_bot(state, actor);
    }
}

// ── Showdown ──────────────────────────────────────────────────────────────────

fn resolve_showdown(state: &mut GameState) {
    let community = state.community_cards.clone();
    let active: Vec<usize> = (0..state.players.len())
        .filter(|&i| state.players[i].status == PlayerStatus::Active)
        .collect();

    if active.len() == 1 {
        let w = active[0];
        state.players[w].stack += state.pot;
        state.result_message = Some(format!("{} wins {} (uncontested)", state.players[w].name, state.pot));
        state.hand_over = true;
        state.hero_to_act = false;
        return;
    }

    let mut best_rank = None;
    let mut winners: Vec<usize> = vec![];

    for &i in &active {
        if let Some(hole) = &state.players[i].hole_cards {
            let cards: Vec<RsCard> = hole.iter().chain(community.iter()).map(|c| c.to_rs()).collect();
            let rank = cards.rank();
            match best_rank {
                None => { best_rank = Some(rank); winners = vec![i]; }
                Some(br) if rank > br => { best_rank = Some(rank); winners = vec![i]; }
                Some(br) if rank == br => { winners.push(i); }
                _ => {}
            }
        }
    }

    if winners.len() == 1 {
        let w = winners[0];
        state.players[w].stack += state.pot;
        let cat = best_rank.map(|r| r.category().to_string()).unwrap_or_default();
        state.result_message = Some(format!("{} wins {} — {}", state.players[w].name, state.pot, cat));
    } else {
        let split = state.pot / winners.len() as u32;
        let names: Vec<String> = winners.iter().map(|&i| state.players[i].name.clone()).collect();
        for &w in &winners { state.players[w].stack += split; }
        state.result_message = Some(format!("Split pot ({} each) — {}", split, names.join(", ")));
    }

    state.hand_over = true;
    state.hero_to_act = false;
}

// ── Equity calculator (v0.3.0) ────────────────────────────────────────────────

pub fn calculate_equity(state: &GameState) -> EquityResult {
    let hero = state.players.iter().find(|p| p.is_hero);
    let hero_cards = match hero.and_then(|p| p.hole_cards.as_ref()) {
        Some(c) => c.clone(),
        None => return EquityResult { win: 0.0, tie: 0.0, lose: 100.0 },
    };

    let num_opponents = state.players.iter().filter(|p| !p.is_hero && p.status == PlayerStatus::Active).count();
    if num_opponents == 0 {
        return EquityResult { win: 100.0, tie: 0.0, lose: 0.0 };
    }

    run_monte_carlo(&hero_cards, &state.community_cards, num_opponents, 10_000)
}

pub fn calculate_equity_detail(state: &GameState) -> EquityDetail {
    let hero = state.players.iter().find(|p| p.is_hero);
    let hero_cards = match hero.and_then(|p| p.hole_cards.as_ref()) {
        Some(c) => c.clone(),
        None => return EquityDetail { win: 0.0, tie: 0.0, lose: 100.0, trials: 0, remaining_deck_size: 0, cards_to_come: 0, cards_per_opp: 2, num_opponents: 0, sample_trials: vec![] },
    };
    let num_opponents = state.players.iter().filter(|p| !p.is_hero && p.status == PlayerStatus::Active).count();
    if num_opponents == 0 {
        return EquityDetail { win: 100.0, tie: 0.0, lose: 0.0, trials: 0, remaining_deck_size: 0, cards_to_come: 0, cards_per_opp: 2, num_opponents: 0, sample_trials: vec![] };
    }
    run_monte_carlo_detail(&hero_cards, &state.community_cards, num_opponents, 10_000)
}

fn hand_category_name(cards: &[RsCard]) -> String {
    let r = cards.rank();
    format!("{}", r.category())
}

fn run_monte_carlo_detail(
    hero: &[Card; 2],
    community: &[Card],
    num_opps: usize,
    trials: usize,
) -> EquityDetail {
    let mut rng = rand::rng();

    let mut remaining: Vec<Card> = full_deck()
        .into_iter()
        .filter(|c| !hero.iter().chain(community.iter()).any(|u| u.rank == c.rank && u.suit == c.suit))
        .collect();

    let cards_to_come = 5 - community.len();
    let per_trial = 2 * num_opps + cards_to_come;

    if remaining.len() < per_trial {
        return EquityDetail { win: 50.0, tie: 0.0, lose: 50.0, trials: 0, remaining_deck_size: remaining.len(), cards_to_come, cards_per_opp: 2, num_opponents: num_opps, sample_trials: vec![] };
    }

    let remaining_deck_size = remaining.len();
    let (mut wins, mut ties, mut losses) = (0u32, 0u32, 0u32);
    let mut samples: Vec<SampleTrial> = Vec::new();

    for trial_idx in 0..trials {
        remaining.shuffle(&mut rng);
        let mut idx = 0;

        let mut board = community.to_vec();
        let mut extra: Vec<Card> = Vec::new();
        for _ in 0..cards_to_come {
            extra.push(remaining[idx].clone());
            board.push(remaining[idx].clone());
            idx += 1;
        }

        let hero_rs: Vec<RsCard> = hero.iter().chain(board.iter()).map(|c| c.to_rs()).collect();
        let hero_score = hero_rs.rank();

        let mut best_opp_score = None;
        let mut first_opp_cards: Vec<Card> = Vec::new();
        let mut best_opp_rs: Vec<RsCard> = Vec::new();

        for opp_i in 0..num_opps {
            let c1 = remaining[idx].clone();
            let c2 = remaining[idx + 1].clone();
            idx += 2;
            let opp_rs: Vec<RsCard> = [c1.clone(), c2.clone()].iter().chain(board.iter()).map(|c| c.to_rs()).collect();
            let score = opp_rs.rank();
            if best_opp_score.map_or(true, |b: rs_poker::core::Rank| score > b) {
                best_opp_score = Some(score);
                best_opp_rs = opp_rs;
            }
            if opp_i == 0 {
                first_opp_cards = vec![c1, c2];
            }
        }

        let result = match best_opp_score {
            None => { wins += 1; "Win" },
            Some(b) if hero_score > b => { wins += 1; "Win" },
            Some(b) if hero_score == b => { ties += 1; "Tie" },
            _ => { losses += 1; "Lose" },
        };

        if trial_idx < 5 {
            samples.push(SampleTrial {
                extra_community: extra,
                opponent_cards: first_opp_cards,
                hero_rank: hand_category_name(&hero_rs),
                best_opp_rank: hand_category_name(&best_opp_rs),
                result: result.to_string(),
            });
        }
    }

    let t = trials as f64;
    EquityDetail {
        win: wins as f64 / t * 100.0,
        tie: ties as f64 / t * 100.0,
        lose: losses as f64 / t * 100.0,
        trials,
        remaining_deck_size,
        cards_to_come,
        cards_per_opp: 2,
        num_opponents: num_opps,
        sample_trials: samples,
    }
}

fn run_monte_carlo(
    hero: &[Card; 2],
    community: &[Card],
    num_opps: usize,
    trials: usize,
) -> EquityResult {
    let mut rng = rand::rng();

    // Build remaining deck
    let mut remaining: Vec<Card> = full_deck()
        .into_iter()
        .filter(|c| !hero.iter().chain(community.iter()).any(|u| u.rank == c.rank && u.suit == c.suit))
        .collect();

    let cards_needed = 5 - community.len();
    let cards_per_opp = 2;
    let per_trial = cards_per_opp * num_opps + cards_needed;

    if remaining.len() < per_trial {
        return EquityResult { win: 50.0, tie: 0.0, lose: 50.0 };
    }

    let (mut wins, mut ties, mut losses) = (0u32, 0u32, 0u32);

    for _ in 0..trials {
        remaining.shuffle(&mut rng);
        let mut idx = 0;

        let mut board = community.to_vec();
        // Deal board runout first
        for _ in 0..cards_needed {
            board.push(remaining[idx].clone());
            idx += 1;
        }

        // Evaluate hero
        let hero_cards_rs: Vec<RsCard> = hero.iter().chain(board.iter()).map(|c| c.to_rs()).collect();
        let hero_score = hero_cards_rs.rank();

        // Evaluate opponents
        let mut best_opp = None;
        for _ in 0..num_opps {
            let opp_rs: Vec<RsCard> = [remaining[idx].clone(), remaining[idx + 1].clone()]
                .iter().chain(board.iter()).map(|c| c.to_rs()).collect();
            idx += 2;
            let score = opp_rs.rank();
            best_opp = Some(match best_opp {
                None => score,
                Some(b) => if score > b { score } else { b },
            });
        }

        match best_opp {
            None => wins += 1,
            Some(b) if hero_score > b => wins += 1,
            Some(b) if hero_score == b => ties += 1,
            _ => losses += 1,
        }
    }

    let t = trials as f64;
    EquityResult {
        win: wins as f64 / t * 100.0,
        tie: ties as f64 / t * 100.0,
        lose: losses as f64 / t * 100.0,
    }
}

// ── Hand analyzer (v0.4.0) ────────────────────────────────────────────────────

fn analyze_hand(snapshots: &[DecisionSnapshot]) -> Vec<DecisionVerdict> {
    snapshots.iter().map(|snap| {
        let pot_odds_pct = if snap.call_amount > 0 {
            snap.call_amount as f64 / (snap.pot as f64 + snap.call_amount as f64) * 100.0
        } else {
            0.0
        };

        let eq = run_monte_carlo(&snap.hero_hole_cards, &snap.community_cards, snap.active_opponents, 3_000);
        let equity_pct = eq.win + eq.tie * 0.5;

        let (correct, verdict) = match snap.action_taken.as_str() {
            "fold" => {
                let ok = pot_odds_pct > 0.0 && equity_pct < pot_odds_pct;
                let msg = if pot_odds_pct == 0.0 {
                    format!("Folded with {:.0}% equity — should have checked (no bet was facing you)", equity_pct)
                } else if ok {
                    format!("Good fold — equity {:.0}% was below pot odds {:.0}%", equity_pct, pot_odds_pct)
                } else {
                    format!("Should have called — equity {:.0}% exceeds pot odds {:.0}%", equity_pct, pot_odds_pct)
                };
                (ok, msg)
            }
            "call" => {
                let ok = equity_pct >= pot_odds_pct;
                let msg = if ok {
                    format!("Good call — equity {:.0}% ≥ pot odds {:.0}%", equity_pct, pot_odds_pct)
                } else {
                    format!("Should have folded — equity {:.0}% was below pot odds {:.0}%", equity_pct, pot_odds_pct)
                };
                (ok, msg)
            }
            "check" => {
                let missed_bet = equity_pct > 60.0;
                let msg = if missed_bet {
                    format!("Should have bet for value — {:.0}% equity is strong enough to build the pot", equity_pct)
                } else {
                    format!("Good check — {:.0}% equity doesn't justify betting", equity_pct)
                };
                (!missed_bet, msg)
            }
            action => {
                let ok = equity_pct >= 40.0;
                let msg = if ok {
                    format!("Good aggression — {:.0}% equity supports this {}", equity_pct, action)
                } else {
                    format!("Should have checked — {:.0}% equity is too weak to {} here", equity_pct, action)
                };
                (ok, msg)
            }
        };

        DecisionVerdict {
            street: snap.street.clone(),
            action: snap.action_taken.clone(),
            pot_odds_pct,
            equity_pct,
            verdict,
            correct,
        }
    }).collect()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn compute_legal(players: &[Player], actor: usize, street_bet: u32, big_blind: u32) -> LegalActions {
    let p = &players[actor];
    let call_amount = street_bet.saturating_sub(p.current_bet).min(p.stack);
    let facing_bet = street_bet > 0 && street_bet > p.current_bet;
    let min_bet = big_blind;
    let min_raise = (street_bet * 2).max(street_bet + big_blind);

    LegalActions {
        can_fold: facing_bet,
        can_check: !facing_bet,
        can_call: facing_bet && call_amount > 0,
        call_amount,
        can_bet: !facing_bet && p.stack >= min_bet,
        min_bet,
        can_raise: facing_bet && p.stack >= min_raise,
        min_raise,
        max_amount: p.stack,
    }
}

fn default_legal() -> LegalActions {
    LegalActions {
        can_fold: false, can_check: false, can_call: false, call_amount: 0,
        can_bet: false, min_bet: 0, can_raise: false, min_raise: 0, max_amount: 0,
    }
}
