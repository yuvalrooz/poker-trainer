import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

// ── Types ──────────────────────────────────────────────────────────────────────

interface Card { rank: number; suit: "Clubs" | "Diamonds" | "Hearts" | "Spades" }

interface Player {
  id: number; name: string; position: string; stack: number;
  hole_cards: [Card, Card] | null; is_hero: boolean;
  status: "Active" | "Folded" | "AllIn"; current_bet: number;
}

interface LegalActions {
  can_fold: boolean; can_check: boolean; can_call: boolean; call_amount: number;
  can_bet: boolean; min_bet: number; can_raise: boolean; min_raise: number; max_amount: number;
}

interface ActionRecord { player_name: string; description: string }

interface DecisionVerdict {
  street: string; action: string; pot_odds_pct: number;
  equity_pct: number; verdict: string; correct: boolean;
}

interface EquityResult { win: number; tie: number; lose: number }

interface GameView {
  players: Player[]; community_cards: Card[]; pot: number; street: string;
  action_history: ActionRecord[]; hero_to_act: boolean; hand_over: boolean;
  result_message: string | null; legal_actions: LegalActions;
  analysis: DecisionVerdict[] | null;
}

// ── Card helpers ──────────────────────────────────────────────────────────────

const RANK: Record<number, string> = {
  14:"A",13:"K",12:"Q",11:"J",10:"T",9:"9",8:"8",7:"7",6:"6",5:"5",4:"4",3:"3",2:"2",
};
const SUIT_SYMBOL: Record<string, string> = { Clubs:"♣", Diamonds:"♦", Hearts:"♥", Spades:"♠" };
const isRed = (s: string) => s === "Hearts" || s === "Diamonds";

function CardFace({ card }: { card: Card }) {
  return (
    <div className={`card ${isRed(card.suit) ? "red" : "black"}`}>
      <span className="card-rank">{RANK[card.rank]}</span>
      <span className="card-suit">{SUIT_SYMBOL[card.suit]}</span>
    </div>
  );
}
function CardBack() { return <div className="card card-back" />; }

function HoleCards({ cards }: { cards: [Card, Card] | null }) {
  if (!cards) return <div className="hole-cards"><CardBack /><CardBack /></div>;
  return <div className="hole-cards"><CardFace card={cards[0]} /><CardFace card={cards[1]} /></div>;
}

// ── Player seat ───────────────────────────────────────────────────────────────

function PlayerSeat({ player, isActing }: { player: Player; isActing: boolean }) {
  const folded = player.status === "Folded";
  return (
    <div className={`seat ${player.is_hero ? "hero-seat" : ""} ${folded ? "folded" : ""} ${isActing ? "acting" : ""}`}>
      <div className="seat-name">
        {player.name} <span className="pos-badge">{player.position}</span>
      </div>
      <HoleCards cards={player.hole_cards} />
      <div className="seat-stack">{player.stack.toLocaleString()}</div>
      {player.current_bet > 0 && <div className="bet-chip">{player.current_bet}</div>}
      {folded && <div className="folded-label">FOLDED</div>}
    </div>
  );
}

// ── Equity bar ────────────────────────────────────────────────────────────────

function EquityBar({ eq }: { eq: EquityResult }) {
  return (
    <div className="equity-bar-wrap">
      <div className="equity-bar">
        <div className="eq-win" style={{ width: `${eq.win}%` }} />
        <div className="eq-tie" style={{ width: `${eq.tie}%` }} />
        <div className="eq-lose" style={{ width: `${eq.lose}%` }} />
      </div>
      <div className="equity-labels">
        <span className="eq-win-lbl">W {eq.win.toFixed(1)}%</span>
        <span className="eq-tie-lbl">T {eq.tie.toFixed(1)}%</span>
        <span className="eq-lose-lbl">L {eq.lose.toFixed(1)}%</span>
      </div>
    </div>
  );
}

// ── Action controls ───────────────────────────────────────────────────────────

function ActionControls({ legal, onAction }: { legal: LegalActions; onAction: (a: string, amt?: number) => void }) {
  const minAmt = legal.can_bet ? legal.min_bet : legal.min_raise;
  const [betAmt, setBetAmt] = useState(minAmt);

  useEffect(() => { setBetAmt(legal.can_bet ? legal.min_bet : legal.min_raise); }, [legal]);

  const showSizer = legal.can_bet || legal.can_raise;

  return (
    <div className="action-controls">
      {legal.can_fold  && <button className="btn btn-fold"  onClick={() => onAction("fold")}>Fold</button>}
      {legal.can_check && <button className="btn btn-check" onClick={() => onAction("check")}>Check</button>}
      {legal.can_call  && <button className="btn btn-call"  onClick={() => onAction("call")}>Call {legal.call_amount}</button>}
      {showSizer && (
        <div className="bet-sizer">
          <input type="range" min={minAmt} max={legal.max_amount} step={25}
            value={betAmt} onChange={e => setBetAmt(Number(e.target.value))} />
          <input
            type="number" className="bet-input"
            min={minAmt} max={legal.max_amount} value={betAmt}
            onChange={e => {
              const v = Math.max(minAmt, Math.min(legal.max_amount, Number(e.target.value)));
              if (!isNaN(v)) setBetAmt(v);
            }}
          />
          {legal.can_bet   && <button className="btn btn-bet"   onClick={() => onAction("bet", betAmt)}>Bet</button>}
          {legal.can_raise && <button className="btn btn-raise" onClick={() => onAction("raise", betAmt)}>Raise</button>}
        </div>
      )}
    </div>
  );
}

// ── Analysis panel ────────────────────────────────────────────────────────────

function AnalysisPanel({ verdicts }: { verdicts: DecisionVerdict[] }) {
  if (verdicts.length === 0) return <div className="analysis"><p className="no-decisions">No hero decisions recorded.</p></div>;

  const correct = verdicts.filter(v => v.correct).length;
  return (
    <div className="analysis">
      <div className="analysis-header">
        Hand Analysis — {correct}/{verdicts.length} decisions correct
      </div>
      {verdicts.map((v, i) => (
        <div key={i} className={`verdict ${v.correct ? "verdict-ok" : "verdict-bad"}`}>
          <div className="verdict-action">
            <span className="verdict-street">{v.street}</span>
            <span className={`verdict-tag ${v.correct ? "tag-ok" : "tag-bad"}`}>
              {v.correct ? "✓" : "✗"} {v.action}
            </span>
          </div>
          <div className="verdict-text">{v.verdict}</div>
          {v.pot_odds_pct > 0 && (
            <div className="verdict-nums">
              Equity {v.equity_pct.toFixed(1)}% · Pot odds {v.pot_odds_pct.toFixed(1)}%
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

// ── Action log ────────────────────────────────────────────────────────────────

function ActionLog({ history }: { history: ActionRecord[] }) {
  return (
    <div className="action-log">
      {history.map((r, i) => (
        <div key={i} className={`log-line ${r.player_name === "" ? "log-street" : ""}`}>
          {r.player_name ? <strong>{r.player_name} </strong> : null}{r.description}
        </div>
      ))}
    </div>
  );
}

// ── App ───────────────────────────────────────────────────────────────────────

export default function App() {
  const [game, setGame] = useState<GameView | null>(null);
  const [equity, setEquity] = useState<EquityResult | null>(null);
  const [equityOn, setEquityOn] = useState(false);
  const [loading, setLoading] = useState(false);
  const [showAnalysis, setShowAnalysis] = useState(false);

  const fetchEquity = useCallback(async () => {
    try {
      const eq = await invoke<EquityResult>("get_equity");
      setEquity(eq);
    } catch { setEquity(null); }
  }, []);

  // Re-fetch equity whenever game state changes and toggle is on
  useEffect(() => {
    if (equityOn && game && !game.hand_over) {
      setEquity(null);
      fetchEquity();
    }
  }, [game, equityOn, fetchEquity]);

  async function startHand() {
    setLoading(true);
    setEquity(null);
    setShowAnalysis(false);
    const view = await invoke<GameView>("new_hand");
    setGame(view);
    setLoading(false);
  }

  async function act(action: string, amount?: number) {
    if (!game || !game.hero_to_act) return;
    setLoading(true);
    setEquity(null);
    const view = await invoke<GameView>("take_action", { action, amount });
    setGame(view);
    if (view.hand_over) setShowAnalysis(true);
    setLoading(false);
  }

  if (!game) {
    return (
      <div className="lobby">
        <h1 className="logo-title">Poker Trainer</h1>
        <p className="lobby-sub">Texas Hold'em · 3–6 players · Randomised scenarios</p>
        <button className="btn btn-deal" onClick={startHand} disabled={loading}>
          {loading ? "Dealing…" : "Deal Hand"}
        </button>
      </div>
    );
  }

  const hero = game.players.find(p => p.is_hero)!;
  const opponents = game.players.filter(p => !p.is_hero);
  const botActing = !game.hero_to_act && !game.hand_over;

  return (
    <div className="table-wrap">
      {/* Opponents row */}
      <div className="opponents-row">
        {opponents.map(p => <PlayerSeat key={p.id} player={p} isActing={botActing} />)}
      </div>

      {/* Board */}
      <div className="board-area">
        <div className="street-label">{game.street}</div>
        <div className="community-cards">
          {game.community_cards.map((c, i) => <CardFace key={i} card={c} />)}
        </div>
        <div className="pot-label">Pot: {game.pot.toLocaleString()}</div>
        {equityOn && equity && (
          <>
            <EquityBar eq={equity} />
            {game.hero_to_act && game.legal_actions.can_call && (() => {
              const potOdds = game.legal_actions.call_amount /
                (game.pot + game.legal_actions.call_amount) * 100;
              const hasEdge = (equity.win + equity.tie * 0.5) >= potOdds;
              return (
                <div className="pot-odds-row">
                  <span className="pot-odds-label">
                    Pot odds: <strong>{potOdds.toFixed(1)}%</strong>
                  </span>
                  <span className={`pot-odds-verdict ${hasEdge ? "odds-good" : "odds-bad"}`}>
                    {hasEdge ? "▲ call has +EV" : "▼ call is -EV"}
                  </span>
                </div>
              );
            })()}
          </>
        )}
        {equityOn && !equity && !game.hand_over && <div className="eq-loading">Calculating…</div>}
      </div>

      {/* Hero row */}
      <div className="hero-row">
        <PlayerSeat player={hero} isActing={game.hero_to_act} />
      </div>

      {/* Controls */}
      <div className="bottom-panel">
        {game.hand_over ? (
          <div className="result-area">
            <div className="result-msg">{game.result_message}</div>
            <div className="result-btns">
              <button className="btn btn-deal" onClick={startHand}>New Hand</button>
              {game.analysis && game.analysis.length > 0 && (
                <button className="btn btn-analyze" onClick={() => setShowAnalysis(v => !v)}>
                  {showAnalysis ? "Hide Analysis" : "Show Analysis"}
                </button>
              )}
            </div>
          </div>
        ) : game.hero_to_act ? (
          <div className="hero-controls">
            <ActionControls legal={game.legal_actions} onAction={act} />
            <button
              className={`btn btn-equity ${equityOn ? "equity-on" : ""}`}
              onClick={() => setEquityOn(v => !v)}
            >
              {equityOn ? "Odds ON" : "Odds OFF"}
            </button>
          </div>
        ) : (
          <div className="waiting">Opponents thinking…</div>
        )}
      </div>

      {/* Analysis (post-hand) */}
      {showAnalysis && game.analysis && (
        <div className="analysis-overlay" onClick={() => setShowAnalysis(false)}>
          <div onClick={e => e.stopPropagation()}>
            <button className="analysis-close" onClick={() => setShowAnalysis(false)}>✕</button>
            <AnalysisPanel verdicts={game.analysis} />
          </div>
        </div>
      )}

      {/* Action log */}
      <ActionLog history={game.action_history} />
    </div>
  );
}
