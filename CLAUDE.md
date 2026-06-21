# Poker Trainer — Project Guide

## What this is
A desktop poker training app. Each session generates a randomized Texas Hold'em
situation (table size, stack depths, street) and the user plays it out with
standard actions. After the hand, the app analyzes the decisions made. This
pairs with ongoing poker strategy training happening outside this codebase —
the analysis module should eventually speak the same language as that
training (pot odds, equity, position, player types).

## Tech stack
- **Frontend**: React + TypeScript, built via Tauri's React-TS template
- **Backend**: Rust, via Tauri 2.0 commands
- **Hand evaluation**: `rs_poker` crate — do not hand-roll a 7-card evaluator
- **Equity calculation**: Monte Carlo simulation in Rust (sample remaining
  deck + opponent ranges, run N trials, return win/tie/lose %)
- **State management (frontend)**: React state/hooks is enough at this scale;
  don't reach for Redux/Zustand unless the game state genuinely outgrows it

## Why Rust for the poker logic
Equity calculation is the one part of this app that's actually
computationally meaningful (thousands of Monte Carlo trials per odds toggle).
Doing it in Rust via Tauri commands keeps the UI responsive and gives a
single source of truth for hand state that both the engine and the eventual
analysis module can trust. The frontend should be a thin rendering/input
layer — it should never duplicate poker logic (e.g. don't recalculate equity
or validate legal actions in TypeScript; always call into Rust).

## Core domain model (src-tauri/src/poker/)
```rust
enum Suit { Clubs, Diamonds, Hearts, Spades }
struct Card { rank: u8, suit: Suit } // rank 2-14, Ace high

enum Street { Preflop, Flop, Turn, River, Showdown }

enum PlayerStatus { Active, Folded, AllIn }

struct Player {
    id: usize,
    stack: u32,
    hole_cards: Option<[Card; 2]>, // None for opponents unless revealed
    is_hero: bool,
    position: Position, // Button, SB, BB, UTG, etc.
    status: PlayerStatus,
    current_bet: u32, // amount committed this street
}

enum Action {
    Fold,
    Check,
    Call,
    Bet(u32),
    Raise(u32),
}

struct GameState {
    players: Vec<Player>,
    community_cards: Vec<Card>,
    pot: u32,
    street: Street,
    button_position: usize,
    small_blind: u32,
    big_blind: u32,
    action_history: Vec<(usize, Action)>, // (player_id, action)
    hero_to_act: bool,
}
```

## Four core systems

1. **Scenario generator** — randomizes player count (2–9), stack depths
   (e.g. 20–150bb), and which street the hero is dropped into. Must produce
   an internally consistent state: if starting on the turn, prior streets'
   action history, pot size, and remaining stacks all need to make sense
   together. Use simple, deterministic bot logic to simulate prior streets'
   action (no need for sophisticated opponent AI here — just plausible
   pot-building).

2. **Equity engine** — given the current `GameState`, run Monte Carlo
   simulation over the remaining deck to estimate hero's win/tie/lose %
   against the live opponents' ranges. Exposed via a Tauri command the
   frontend calls when the odds toggle is on. Should be fast enough to
   re-run after every street without UI lag (target: <100ms for ~10k trials).

3. **Action engine** — validates and applies legal actions given game state
   (e.g. can't check facing a bet, raise must meet minimum). This is the
   state machine that advances `GameState` street-by-street to showdown or
   early fold.

4. **Hand analyzer** — runs after the hand resolves. For each decision point,
   compares the action taken against pot odds and computed equity at that
   moment, and produces a verdict (e.g. "called with insufficient equity,"
   "missed a value-raise spot"). Statistical analysis lives in Rust; if/when
   this hooks into the Claude API for narrative coaching feedback, that call
   happens from the frontend or a separate Tauri command — keep API key
   handling out of version control (use Tauri's secure storage, not env
   files checked into git).

See `docs/POKER_ENGINE_SPEC.md` for more detail on the equity algorithm and
scenario generation logic.

## Version roadmap
- **v0.1.0** — one hardcoded scenario, fold/check/call/bet/raise fully
  working end to end, no randomization, no odds toggle, no analysis. This is
  the "static app" baseline.
- **v0.2.0** — scenario generator: randomize players/stacks/street
- **v0.3.0** — equity odds toggle in the UI
- **v0.4.0** — post-hand analysis panel

Don't skip ahead — each version should be a playable, committable
milestone. Tag releases (`git tag v0.1.0`) once a milestone is working.

## Conventions
- Commit style: conventional commits (`feat:`, `fix:`, `chore:`, `refactor:`)
- Poker logic gets unit tests in Rust (`cargo test`) — especially hand
  evaluation edge cases (wheel straights, flush vs straight, kicker
  comparisons) and equity calc sanity checks (known hand matchups with
  published equity %)
- No poker logic in the frontend — TypeScript only renders state and sends
  actions to Rust

## Open decisions for future sessions
- Opponent range modeling for equity calc: start with "any two cards"
  uniform random, refine to position-based ranges later
- Whether bot action during scenario generation should ever be visible to
  the user as a "prior action" summary, or just reflected in pot/stack state
