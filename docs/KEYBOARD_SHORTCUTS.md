# TyphooN Terminal -- Keyboard Shortcut Reference

All shortcuts defined in `frontend/src/main.js` (`setupKeyboard()` and global listeners).

---

## Context Sensitivity

Keyboard shortcuts are **disabled** when focus is inside an `<input>` or `<select>` element
(symbol search, SL/TP fields, command palette input, etc.). The `setupKeyboard()` handler
exits immediately if `e.target.tagName === "INPUT" || e.target.tagName === "SELECT"`.

The global `Ctrl+` combinations and `Ctrl+Shift+` combinations fire regardless of focus
because they are registered in separate listeners that do not have the input guard.

---

## Function Keys (Trading)

| Key  | Action                                      |
|------|---------------------------------------------|
| F1   | Help & Keybindings overlay (toggle)         |
| F2   | Buy Lines (places SL low / TP high)         |
| F3   | Sell Lines (places SL high / TP low)        |
| F4   | Open Trade (calculates lots, places order)  |
| F5   | Destroy SL/TP Lines                         |
| F6   | Cycle Martingale Mode (OFF/LONG/SHORT)      |
| F7   | Close All Positions                         |
| F8   | Close Partial Position                      |

---

## Ctrl+ Combinations

| Key             | Action                                    |
|-----------------|-------------------------------------------|
| Ctrl+K          | Open/close Command Palette                |
| Ctrl+T          | New Tab                                   |
| Ctrl+W          | Close current Tab                         |
| Ctrl+Shift+S    | Screenshot (capture chart to clipboard)   |

---

## Drawing Tools (single lowercase key)

These activate a drawing mode; click the chart to place anchor points.

| Key     | Tool                                       |
|---------|--------------------------------------------|
| L       | Trend Line (click two points)              |
| F       | Fibonacci Retracement (click high/low)     |
| H       | Horizontal Line (click to place)           |
| R       | Rectangle (click two corners)              |
| E       | Ray (extends right from two points)        |
| C       | Channel (parallel lines, 3 clicks)         |
| Delete  | Remove last drawing                        |

---

## General / Navigation

| Key     | Action                                          |
|---------|-------------------------------------------------|
| ?       | Help overlay (same as F1)                       |
| Escape  | Clear SL/TP lines; close help overlay           |
| /       | Open Command Palette (same as Ctrl+K)           |

---

## Alt+ Window Management

| Key     | Action                          |
|---------|---------------------------------|
| Alt+W   | Close all floating windows      |
| Alt+G   | Tile all floating windows       |

---

## Menu Bar Hints (click only)

The menu bar labels show parenthetical key hints (e.g., "Buy Lines (B)",
"Sell Lines (S)"). These are **menu label hints for reference only** -- they
indicate the first letter of the action but are not wired as standalone
keyboard shortcuts. The actual keyboard equivalents are the F-keys listed
above or the drawing-tool keys.

| Menu Label      | Hint | Actual Keyboard Equivalent   |
|-----------------|------|------------------------------|
| Buy Lines       | (B)  | F2                           |
| Sell Lines      | (S)  | F3                           |
| Destroy Lines   | (D)  | F5                           |
| Open Trade      | (T)  | F4                           |
| Close All       | (C)  | F7                           |
| Close Partial   | (P)  | F8                           |
| Trend Line      | (L)  | L                            |
| Fibonacci       | (F)  | F                            |
| Ray             | (E)  | E                            |
| Ruler           | (J)  | Menu only (no hotkey)        |
| Horizontal Line | (N)  | H (code uses `h`, not `n`)   |
| Rectangle       | (R)  | R                            |
| Set Alert       | (A)  | Menu only (no hotkey)        |
| Delete Drawing  | (X)  | Delete key                   |

---

## Command Palette Commands

Open with **Ctrl+K** or **/** then type any of these:

| Command     | Description                                |
|-------------|--------------------------------------------|
| DES         | Company fundamentals (SEC EDGAR)           |
| NEWS        | News headlines                             |
| FA          | Financial analysis (income/balance/cash)   |
| OPT         | Options chain (Greeks, bid/ask)            |
| SCAN        | Stock screener                             |
| BACKTEST    | Visual backtester (SMA Cross, NNFX)       |
| OPTIMIZE    | Grid search optimizer                      |
| OPTCALC     | Options P&L calculator (payoff diagram)    |
| OPTSTRAT    | Options strategy builder                   |
| SECTORS     | Sector rotation heatmap (S&P 500 ETFs)    |
| ECON        | Economic calendar with countdown           |
| CHAT        | Community chat (Matrix protocol)           |
| AUTOTRADE   | Strategy auto-trading framework            |
| ALERTS      | Multi-condition alert manager              |
| ALERTBOARD  | Multi-symbol alert dashboard               |
| PORTFOLIO   | Portfolio breakdown by sector              |
| CORR        | Correlation matrix                         |
| MONTECARLO  | Monte Carlo risk of ruin                   |
| PATTERNS    | Pattern recognition (H&S, Double Top)      |
| SENTIMENT   | News sentiment analysis                    |
| AI          | AI trading assistant                       |
| SETTINGS    | API keys & configuration                   |
| TILE        | Tile all floating windows                  |
| CLOSE       | Close all floating windows                 |
| HELP        | Help screen                                |

Inside the palette: **Arrow Up/Down** to navigate, **Enter** to select, **Escape** to close.

---

## Input-Specific Shortcuts

These work **only** when the respective input field has focus:

| Context              | Key   | Action                               |
|----------------------|-------|--------------------------------------|
| Symbol input         | Enter | Load symbol / confirm autocomplete   |
| Symbol input         | Esc   | Close autocomplete dropdown          |
| Symbol input         | Up/Dn | Navigate autocomplete suggestions    |
| SL price input       | Enter | Set stop-loss line + backend order   |
| TP price input       | Enter | Set take-profit line + backend order |
| Secret Key (login)   | Enter | Connect to broker                    |
| Watchlist add input  | Enter | Add symbol to watchlist              |
| AI / Chat input      | Enter | Send message                         |

---

## Help Overlay Close

The help overlay (opened by **F1** or **?**) can be closed by:

- Pressing **Escape**, **?**, or **F1** again
- Clicking outside the panel
