# Poker Engine Spec

## Scenario generation algorithm

1. Pick `num_players` uniformly from 2–9.
2. Assign each player a stack depth in big blinds, e.g. uniform random
   20–150bb, converted to chips at a fixed blind level (e.g. 50/100).
3. Pick a starting `street` for the hero to be dropped into (weight toward
   Preflop/Flop early on; all four streets should be reachable).
4. Deal hole cards to all players (only hero's are revealed to the UI).
5. If starting street is past Preflop, deal community cards for the
   street(s) already passed, and simulate simplified prior action:
   - Each non-hero player either folds, calls, or raises based on a crude
     hand-strength heuristic (no need for realism beyond "plausible").
   - Track resulting pot size and remaining active players.
   - Stop simulating once street == starting street, with action now at
     the hero.
6. Validate: pot, stacks, and active player count must be self-consistent
   (stacks = starting stack − amount committed; pot = sum of commitments).

## Equity calculation (Monte Carlo)

Given the hero's hole cards, the known community cards, and which
opponents are still active:

1. Build the remaining deck (52 cards minus hero's hole cards minus known
   community cards).
2. For N trials (start with 10,000, tune for speed):
   - Deal random hole cards to each active opponent from the remaining
     deck.
   - Deal random remaining community cards to complete the board.
   - Evaluate all hands with `rs_poker`, determine winner(s).
   - Tally win / tie / loss for hero.
3. Return win%, tie%, loss% to the frontend.

For heads-up preflop, consider precomputed/exact equity tables later as an
optimization — not needed for v0.3.

## Hand analysis (post-hand)

For each decision point in `action_history` where the hero acted:

1. Reconstruct the `GameState` at that moment (pot size, bet facing hero,
   stacks).
2. Compute pot odds: `amount_to_call / (pot + amount_to_call)`.
3. Compute hero's equity at that moment (same Monte Carlo engine).
4. Compare: if equity > pot odds required, a call was justified; flag
   over-folds and over-calls relative to that threshold.
5. Surface a per-decision verdict and an overall hand summary (e.g. "3 of 4
   decisions aligned with pot odds; missed value on the river raise").

This is intentionally a simple, explainable heuristic (pot-odds-vs-equity),
not a full GTO solver — that's a reasonable v0.4 scope. Anything more
sophisticated (range-vs-range solving, exploitative adjustments) is future
work, not part of this baseline.
