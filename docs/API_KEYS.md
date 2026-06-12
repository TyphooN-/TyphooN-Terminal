# TyphooN-Terminal — API Keys and Broker Credentials

Optional API keys and broker credentials that unlock additional features.

## Alpaca Markets (Optional Broker/Data Source)

**Used for:** Alpaca trading, bar data, news, options, corporate actions, WebSocket streaming

- **Sign up:** https://app.alpaca.markets/signup
- **Paper trading:** Free, instant activation
- **Live trading:** Requires identity verification
- **Data:** IEX (free) or SIP (paid) market data feeds
- **Rate limit:** 200 requests/minute (free plan)

## FRED (Federal Reserve Economic Data)

**Used for:** Historical market statistics, interest rates, GDP, CPI, unemployment, yield curves

- **Sign up:** https://fred.stlouisfed.org/docs/api/api_key.html
- **Cost:** Free
- **Rate limit:** 120 requests/minute
- **Key format:** 32-character alphanumeric string

## Kraken (Primary Market Data + Trading)

**Used for:** Public Spot and Kraken Equities/xStocks market data, public Kraken Futures instrument/candle data, public tradeable-pair discovery, and authenticated crypto/xStocks balances, open orders, open positions, order placement, amend/edit, cancel-all, and batch orders.

- **Sign up:** https://pro.kraken.com/
- **Cost:** Free API keys; trading fees apply on filled orders
- **Key format:** API key + base64 API secret
- **Required permissions:** Balance/positions for account display; order create/modify and cancel/close for trading
- **Note:** Public Spot OHLCV, Kraken Equities/xStocks iapi market data, Kraken Futures instrument discovery, and Kraken Futures chart candles do not require credentials. The terminal syncs these bars asynchronously but paces Spot OHLC requests to Kraken's documented public limits and Kraken iapi requests through the AIMD limiter; authenticated keys are only needed for trading/account features. Kraken market-data credentials do not expand news coverage — news comes from the separate multi-source research pipeline in ADR-078.

## Anthropic (Claude AI)

**Used for:** AI chat — natural language queries about market data, position analysis

- **Sign up:** https://console.anthropic.com/
- **Cost:** Pay-per-use (Haiku ~$0.25/1M tokens, Sonnet ~$3/1M tokens)
- **Key format:** `sk-ant-...`

## OpenAI (GPT)

**Used for:** AI chat — alternative to Claude

- **Sign up:** https://platform.openai.com/signup
- **Cost:** Pay-per-use (GPT-4o-mini ~$0.15/1M tokens)
- **Key format:** `sk-...`

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
