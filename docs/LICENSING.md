# TyphooN-Terminal Licensing Analysis

## Current License: Apache-2.0

Both repositories use Apache License 2.0:
- `MQL5-NNFX-Risk_Management_System` — Apache-2.0
- `TyphooN-Terminal` — Apache-2.0

## License Comparison for Open Source Trading Software

### GPL-3.0 (Current)
**Pros:**
- Strong copyleft — all derivatives must also be GPL-3.0
- Prevents proprietary forks (no one can take your code closed-source)
- Patent protection clause (contributors grant patent license)
- Community trust — signals commitment to open source

**Cons:**
- **Commercial poison pill** — any company that links/embeds GPL code must open-source their entire product
- **Broker partnerships blocked** — if Alpaca wanted to integrate/acquire, they'd have to GPL their entire platform or negotiate a separate license
- **SaaS loophole** — GPL doesn't apply to server-side use (someone could run it as a service without sharing changes)
- **Contributor friction** — some contributors avoid GPL due to employer IP policies
- **Library incompatibility** — can't link with Apache-2.0-only or proprietary libraries without conflict

### AGPL-3.0 (Stronger copyleft)
**Pros:**
- Closes the SaaS loophole — network use triggers copyleft
- Prevents someone from hosting TyphooN-Terminal as a web service without sharing code
- Strongest protection for the original author

**Cons:**
- Even more restrictive than GPL — scares away all commercial interest
- Most companies have blanket bans on AGPL
- Would make Alpaca partnership virtually impossible

### Apache-2.0 (Permissive)
**Pros:**
- **Acquisition-friendly** — if Alpaca wants to acquire or partner, no license conflict
- Patent grant protects users and contributors
- Compatible with GPL (can be combined with GPL code)
- Most companies prefer Apache-2.0 contributors

**Cons:**
- No copyleft — anyone can fork and go proprietary (close-source it)
- Competitors could take the codebase and commercialize without contributing back
- Less "community protection" than GPL

### MIT (Most permissive)
**Pros:**
- Simplest license — universally accepted by all companies
- Maximum adoption potential
- TradingView lightweight-charts (which we embed) is MIT

**Cons:**
- No patent protection (unlike Apache-2.0)
- No copyleft at all — proprietary forks are unrestricted
- Minimal legal protection for the author

### BSL (Business Source License) / Source-Available
**Pros:**
- Code is viewable by everyone (transparent)
- **Prevents commercial competition** for a set period (e.g., 3 years)
- After the period, converts to a permissive license (e.g., Apache-2.0)
- Used by: MariaDB, CockroachDB, Sentry, Materialize
- **Best of both worlds**: community can read/learn/contribute, but can't compete commercially

**Cons:**
- Not technically "open source" by OSI definition
- Some community backlash (perceived as "fauxpen source")
- Complexity in defining what counts as "commercial use"

### Dual Licensing (GPL + Commercial)
**Pros:**
- **Community version is GPL** — free for open source use
- **Commercial license available** — companies pay for proprietary use rights
- Revenue potential without losing open source community
- Used by: Qt, MySQL (Oracle), MongoDB (pre-SSPL)

**Cons:**
- Requires all contributors to sign a CLA (Contributor License Agreement)
- Adds legal/administrative overhead
- Community may feel exploited if dual-license isn't disclosed upfront

## Recommendation for TyphooN-Terminal

### If the goal is maximum adoption + Alpaca/broker partnership:
**Switch to Apache-2.0**
- Alpaca could integrate TyphooN-Terminal as an official client
- Brokers could embed the risk engine in their platforms
- Contributors and companies both welcome

### If the goal is revenue + protection:
**Switch to BSL with Apache-2.0 conversion (3 years)**
- Source is readable, community can contribute
- No one can compete commercially for 3 years
- Converts to fully open after 3 years

### If the goal is keep current approach:
**Stay GPL-3.0 but consider dual licensing**
- Add a CLA for contributors
- Offer commercial licenses on request
- Alpaca could negotiate a separate commercial license

## CFTC / SEC Regulatory Considerations

### CFTC (Commodity Futures Trading Commission)
- **Applies to:** Futures, forex, swaps, crypto derivatives
- **Risk:** If TyphooN-Terminal provides automated trading advice (the EA/martingale logic), it could be considered a CTA (Commodity Trading Advisor) or CPO (Commodity Pool Operator)
- **Mitigation:** Disclaimer that this is educational software, not financial advice. The existing disclaimer in README is good.
- **No registration needed** if: the software is a tool (like a calculator) not an advisory service

### SEC (Securities and Exchange Commission)
- **Applies to:** Stocks, ETFs, options
- **Risk:** If you charge for the software AND it provides trade recommendations, it could be considered investment advice requiring RIA registration
- **Mitigation:** Free and open source software is not advisory. The EA doesn't recommend specific trades — it calculates risk.

### NFA (National Futures Association)
- **Applies to:** Forex, futures trading
- **Risk:** Similar to CFTC — automated trading systems that manage client money need NFA registration
- **Mitigation:** TyphooN-Terminal trades the user's own account, not managed accounts

### Recommended Disclaimers
The current README disclaimer is good:
> "This software is provided for educational and research purposes. Trading involves risk."

Consider adding:
> "This software is not a registered investment advisor, broker-dealer, or CTA. It does not provide personalized investment advice. Past performance does not guarantee future results. You are solely responsible for your own trading decisions."

## Alpaca Partnership Scenario

If Alpaca approached to partner/acquire:

### Under GPL-3.0 (Current):
- Alpaca would need to either:
  1. Open-source any product that integrates TyphooN-Terminal
  2. Negotiate a separate commercial license with you
  3. Keep it as a standalone recommended tool (not integrated)

### Under Apache-2.0:
- Alpaca could freely integrate the risk engine into their web/mobile apps
- They could hire you as a consultant/employee and use the code
- No license negotiation needed

### Under BSL:
- Alpaca could use it internally during the BSL period by purchasing a commercial license
- After conversion to Apache-2.0, they could use it freely

### Under Dual License (GPL + Commercial):
- Alpaca buys a commercial license, uses code in their proprietary platform
- Community still gets the GPL version for free
- You earn revenue from the commercial license
