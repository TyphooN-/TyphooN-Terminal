# TyphooN-Terminal — API Keys and Broker Credentials

Optional API keys and broker credentials that unlock additional features.

## Alpaca Markets (Optional Broker/Data Source)

**Used for:** Alpaca trading, bar data, news, options, corporate actions, WebSocket streaming

- **Sign up:** https://app.alpaca.markets/signup
- **Paper trading:** Free, instant activation
- **Live trading:** Requires identity verification
- **Data:** IEX (free) or SIP (paid) market data feeds
- **Rate limit:** 200 requests/minute (free plan), enforced **per account key**
- **Multiple accounts (ADR-130):** the free tier allows 1 live + 3 paper
  accounts. Settings → API Keys exposes 4 identical Alpaca slots — Key,
  Secret, and a Paper/Live mode each. Every **successfully connected** slot
  joins the historical bar-sync rotation and can trade; invalid/disconnected
  slots are excluded. Four connected accounts can provide roughly 4× aggregate
  historical request capacity because each key has its own limiter. Rotation is
  per dispatched request/batch, independent of the selected Primary account;
  all accounts write the same canonical `alpaca:SYM:TF` cache namespace rather
  than fetching duplicate copies. Slot 1 uses the legacy
  `alpaca_api_key`/`alpaca_secret` keyring
  entries; slots 2–4 store under `alpaca_api_key_N`/`alpaca_secret_N`.
  Credentials save to the keyring as soon as the field is edited — no
  Connect click needed. The top-bar `Primary:` chip cycles accounts; the
  `TRADECOPY` console command (or Trading → TradeCopy…) copies positions
  between accounts and can mirror new app-placed orders — mirroring is
  strictly opt-in per target account, never persists across restarts, and
  live accounts must additionally be unlocked as targets.

## FRED (Federal Reserve Economic Data)

**Used for:** Historical market statistics, interest rates, GDP, CPI, unemployment, yield curves

- **Sign up:** https://fred.stlouisfed.org/docs/api/api_key.html
- **Cost:** Free
- **Rate limit:** 120 requests/minute
- **Key format:** 32-character alphanumeric string

## Kraken (Primary Market Data + Trading)

**Used for:** Public Spot and Kraken Equities/xStocks market data, public Kraken Futures instrument/candle data, public tradeable-pair discovery, and authenticated crypto/xStocks balances, open orders, open positions, order placement, amend/edit, cancel-all, and batch orders. Kraken Futures trading surface is lighter (primarily data for chart sync; full private trading gated by entitlements).

- **Sign up:** https://pro.kraken.com/
- **Cost:** Free API keys; trading fees apply on filled orders
- **Key format:** API key + base64 API secret
- **Required permissions:** Balance/positions for account display; order create/modify and cancel/close for trading
- **Note:** Public Spot OHLCV, Kraken Equities/xStocks iapi market data, Kraken Futures instrument discovery, and Kraken Futures chart candles do not require credentials. The terminal syncs these bars asynchronously but paces Spot OHLC requests to Kraken's documented public limits and Kraken iapi requests through the AIMD limiter; authenticated keys are only needed for trading/account features. Kraken market-data credentials do not expand news coverage — news comes from the separate multi-source research pipeline in ADR-078.
- **Multiple accounts (ADR-130):** Settings exposes Kraken slots 2–4
  (`kraken_api_key_N`/`kraken_api_secret_N`) as additional trading identities
  for the top-bar `Primary:` account cycler — Key and Secret per slot, saved
  to the keyring on edit. Because Kraken market data is public, extra Kraken
  accounts do **not** increase sync speed. The private ownTrades/openOrders
  WebSocket re-authenticates to the new account automatically on a primary
  switch, and `TRADECOPY` can one-shot copy spot xStock holdings between
  Kraken accounts (margin positions are skipped; every Kraken account is
  treated as LIVE by the safety rails).

## Anthropic (Claude AI)

**Used for:** AI chat — natural language queries about market data, position analysis

- **Sign up:** https://console.anthropic.com/
- **Cost:** Pay-per-use (input, per 1M tokens: Haiku 4.5 ~$1, Sonnet 5 ~$3,
  Opus 4.8 ~$5 — see https://platform.claude.com/docs/en/pricing for current rates)
- **Key format:** `sk-ant-...`

## OpenAI (GPT)

**Used for:** AI chat — alternative to Claude

- **Sign up:** https://platform.openai.com/signup
- **Cost:** Pay-per-use — see https://openai.com/api/pricing for current rates
- **Key format:** `sk-...`

## Gemini (Google), Grok (xAI), Mistral, Perplexity

**Used for:** AI chat — alternative hosted providers selectable in the AI
Assistant window (Settings has a key field for each; local Ollama / LM Studio
need no key)

- **Sign up:** https://aistudio.google.com/ (Gemini), https://console.x.ai/
  (Grok), https://console.mistral.ai/ (Mistral), https://www.perplexity.ai/settings/api
  (Perplexity)
- **Cost:** Pay-per-use per provider
- **Key format:** provider-specific token string

## CryptoPanic

**Used for:** Crypto news headlines in the multi-source news pipeline (ADR-078)

- **Sign up:** https://cryptopanic.com/developers/api/ (free tier)
- **Key format:** API token string

## Matrix (Community Chat)

**Used for:** `CHAT` command — Matrix-protocol community chat and Matrix
notifications (ADR-053)

- **Setup:** paste a Matrix access token + user id in Settings ("Save Matrix
  Token" stores them in the keyring); any homeserver account works
- **Cost:** Free

## Pushover (Mobile Push Notifications)

**Used for:** Trading alerts sent to your phone

- **Sign up:** https://pushover.net/
- **Cost:** $5 one-time purchase (30-day trial free)
- **Key format:** Application token + User key (30 chars each)

## ntfy.sh (Free Push Notifications)

**Used for:** Trading alerts sent to phone/desktop

- **Sign up:** https://ntfy.sh/ (no account needed)
- **Cost:** Free (self-hosted or public server)
- **Usage:** Subscribe to a topic in the ntfy app, enter topic name in terminal

## Congress Trading Data (QuiverQuant)

**Used for:** Congressional stock trades (House + Senate disclosures)

- **Sign up:** https://www.quiverquant.com/
- **Cost:** Free tier available
- **Key format:** API token string

## Finnhub

**Used for:** Analyst ratings, short interest, IPO calendar, earnings, insider sentiment, company news

- **Sign up:** https://finnhub.io/register
- **Cost:** Free tier: 60 calls/minute
- **Key format:** API token string

## Financial Modeling Prep (FMP)

**Used for:** Analyst estimates, financial ratios, DCF valuation, company profiles

- **Sign up:** https://financialmodelingprep.com/developer
- **Cost:** Free tier: 250 calls/day
- **Key format:** API token string

## Alpha Vantage

**Used for:** Earnings surprises, company overview, fundamental data

- **Sign up:** https://www.alphavantage.co/support/#api-key
- **Cost:** Free tier: 25 calls/day
- **Key format:** API token string

---

## How to Add Keys

In TyphooN-Terminal: **~ → SETTINGS** to open the settings panel. Enter your API keys — they're stored in the OS-native keyring (libsecret on Linux, Keychain on macOS, Credential Manager on Windows).
