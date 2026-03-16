# TyphooN-Terminal — Free API Keys

Optional API keys that unlock additional features. All are free to register.

## Alpaca Markets (Required)

**Used for:** Trading, bar data, news, options, corporate actions, WebSocket streaming

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

---

## How to Add Keys

In TyphooN-Terminal: **Ctrl+K → SETTINGS** to open the settings panel. Enter your API keys — they're stored securely in the OS keychain (gnome-keyring/KWallet).
