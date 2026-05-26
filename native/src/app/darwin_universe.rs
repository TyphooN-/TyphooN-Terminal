//! Darwinex Zero USA Equity Universe (Stocks + ETFs)
#![allow(dead_code)]
//!
//! Full list of tradable USA Stocks and ETFs from Darwinex Zero
//! Source: Market Watch export (2026-05-15) from https://www.darwinexzero.com/assets
//!
//! Filtered out: EURGBP, EURUSD, GBPUSD (not tradable on user's account)
//!
//! Purpose: View and analyze these symbols in TyphooN Terminal using
//! data already available from Kraken / Alpaca. No MT5 sync planned.

use std::collections::HashSet;

/// Full list of USA Stocks + ETFs tradable on Darwinex Zero.
pub const DARWINEX_USA_EQUITY_SYMBOLS: &[&str] = &[
    "A", "AA", "AAL", "AAP", "AAPL", "AAXJ", "ABBV", "ABT", "ACHC", "ACM", "ACN", "ADBE", "ADI",
    "ADM", "ADP", "ADSK", "AEP", "AES", "AFG", "AFL", "AGCO", "AIG", "AIZ", "AJG", "AKAM", "ALB",
    "ALGN", "ALK", "ALL", "ALLY", "ALNY", "ALTR", "AMAT", "AMCR", "AMD", "AME", "AMGN", "AMP",
    "AMT", "AMZN", "ANET", "ANF", "ANSS", "AON", "AOS", "APA", "APD", "APH", "APO", "APP", "APTV",
    "AR", "ARE", "ARKK", "ARKQ", "ARKW", "ARKG", "ARKF", "ARKX", "ARM", "ARW", "ASAN", "ASML",
    "ASO", "ATKR", "ATO", "ATVI", "AVB", "AVGO", "AVY", "AWK", "AXON", "AXP", "AZO", "BA", "BAC",
    "BAX", "BBWI", "BBY", "BC", "BCE", "BCS", "BDX", "BEN", "BF.B", "BG", "BIIB", "BIO", "BK",
    "BKNG", "BKR", "BLDR", "BLK", "BMRN", "BMY", "BN", "BNDX", "BNTX", "BOH", "BOKF", "BOND", "BR",
    "BRK.B", "BRO", "BSX", "BTU", "BURL", "BWA", "BWXT", "BX", "BXP", "BYD", "BYND", "C", "CAG",
    "CAH", "CARR", "CARS", "CASH", "CAT", "CB", "CBOE", "CBRE", "CC", "CCI", "CCL", "CDNS", "CDW",
    "CE", "CEG", "CELH", "CF", "CFG", "CFR", "CG", "CGNX", "CHD", "CHRW", "CHTR", "CI", "CINF",
    "CL", "CLF", "CLX", "CM", "CMA", "CMCSA", "CME", "CMG", "CMI", "CMS", "CNC", "CNP", "CNQ",
    "COF", "COG", "COIN", "COLM", "COO", "COP", "COR", "COST", "COTY", "CPB", "CPNG", "CPRT", "CR",
    "CRL", "CRM", "CRSP", "CSCO", "CSGP", "CSL", "CSX", "CTAS", "CTLT", "CTSH", "CTVA", "CUBE",
    "CUK", "CVS", "CVX", "CW", "CZR", "D", "DAL", "DAN", "DAR", "DB", "DBX", "DD", "DDOG", "DE",
    "DECK", "DEI", "DELL", "DEO", "DFS", "DG", "DGX", "DHI", "DHR", "DIS", "DISCA", "DISCK",
    "DISH", "DLR", "DLTR", "DNB", "DOC", "DOCU", "DOV", "DOW", "DPZ", "DRI", "DTE", "DTM", "DUK",
    "DVA", "DVN", "DXC", "DXCM", "EA", "ECL", "ED", "EFX", "EG", "EIX", "EL", "ELV", "EMN", "EMR",
    "ENB", "ENPH", "ENTG", "EOG", "EPAM", "EPD", "EQIX", "EQR", "EQT", "ERIE", "ES", "ESS", "ETN",
    "ETR", "ETSY", "EVR", "EW", "EWBC", "EXC", "EXEL", "EXPD", "EXPE", "EXR", "F", "FANG", "FAST",
    "FBIN", "FCN", "FCNCA", "FCX", "FDS", "FDX", "FE", "FFIV", "FICO", "FIS", "FITB", "FIVN", "FL",
    "FLEX", "FLO", "FLR", "FLS", "FLT", "FMC", "FMX", "FND", "FNF", "FNV", "FOXA", "FOX", "FR",
    "FRT", "FSLR", "FTNT", "FTV", "FUBO", "FULT", "G", "GDDY", "GE", "GEHC", "GEN", "GILD", "GIS",
    "GL", "GLD", "GLW", "GM", "GME", "GNRC", "GOOG", "GOOGL", "GPC", "GPN", "GPS", "GRMN", "GS",
    "GT", "GWW", "H", "HAL", "HAS", "HBAN", "HBI", "HCA", "HD", "HES", "HIG", "HII", "HLT", "HOLX",
    "HON", "HPE", "HPQ", "HRL", "HSIC", "HST", "HSY", "HUBB", "HUM", "HWM", "IAC", "IART", "IBKR",
    "IBM", "IBN", "ICE", "ICLR", "IDXX", "IEX", "IFF", "ILMN", "INCY", "INDI", "ING", "INTC",
    "INTU", "INVH", "IP", "IPG", "IPGP", "IQV", "IR", "IRM", "ISRG", "IT", "ITW", "IVZ", "J",
    "JBHT", "JBL", "JCI", "JD", "JKHY", "JLL", "JNJ", "JNPR", "JPM", "JWN", "K", "KBR", "KDP",
    "KEY", "KEYS", "KHC", "KIM", "KLAC", "KMB", "KMI", "KMX", "KO", "KR", "KRC", "KSS", "KSU", "L",
    "LAD", "LAMR", "LANC", "LAZ", "LBRDA", "LBRDK", "LBTYA", "LBTYK", "LDO", "LDOS", "LEA", "LECO",
    "LEN", "LFUS", "LGND", "LH", "LHX", "LIN", "LKQ", "LLY", "LMT", "LNC", "LNG", "LNT", "LOW",
    "LPLA", "LRCX", "LSCC", "LSTR", "LSXMA", "LSXMK", "LULU", "LUMN", "LUV", "LVS", "LW", "LYB",
    "LYFT", "LYV", "M", "MA", "MAA", "MAN", "MANH", "MAR", "MAS", "MASI", "MAT", "MCD", "MCHP",
    "MCK", "MCO", "MDLZ", "MDT", "MET", "META", "MGM", "MHK", "MKC", "MKTX", "MLM", "MMC", "MMM",
    "MNST", "MO", "MOH", "MORN", "MOS", "MPC", "MPWR", "MRK", "MRO", "MS", "MSCI", "MSFT", "MSI",
    "MSM", "MTB", "MTCH", "MTD", "MTN", "MTZ", "MU", "MUR", "MUSA", "MXIM", "NDAQ", "NDSN", "NEE",
    "NEM", "NFLX", "NI", "NICE", "NKE", "NLY", "NNN", "NOC", "NOV", "NOW", "NRG", "NSC", "NTAP",
    "NTES", "NTLA", "NTRS", "NUE", "NVDA", "NVR", "NWL", "NWS", "NWSA", "NXPI", "O", "OAS", "ODFL",
    "ODP", "OEF", "OGN", "OHI", "OKE", "OKTA", "OMC", "ON", "ONTO", "ORCL", "ORLY", "OSK", "OTIS",
    "OXY", "OZK", "PAA", "PACW", "PAG", "PANW", "PARA", "PAYC", "PAYX", "PBCT", "PCAR", "PCG",
    "PCTY", "PDCO", "PEG", "PEAK", "PEP", "PFE", "PFG", "PFGC", "PGR", "PG", "PH", "PHM", "PII",
    "PINS", "PIPR", "PK", "PKG", "PKI", "PLD", "PLNT", "PLTR", "PLUG", "PM", "PNC", "PNR", "PNW",
    "POOL", "POR", "POST", "PPC", "PPG", "PPL", "PR", "PRU", "PSA", "PSX", "PTC", "PTON", "PUBM",
    "PVH", "PWR", "PXD", "PYPL", "QCOM", "QGEN", "QRVO", "RCL", "RCM", "REG", "REGN", "RF", "RGA",
    "RGEN", "RGLD", "RHI", "RJF", "RL", "RMD", "RNG", "ROK", "ROL", "ROP", "ROST", "RPRX", "RRC",
    "RRD", "RS", "RSG", "RVTY", "RWE", "RXO", "RY", "RYN", "S", "SAIA", "SAM", "SAND", "SBAC",
    "SBNY", "SBUX", "SCHW", "SCI", "SEDG", "SEE", "SEIC", "SF", "SGEN", "SHW", "SIG", "SIVB",
    "SJM", "SLB", "SLG", "SLM", "SMA", "SMAR", "SMCI", "SMD", "SMFG", "SMG", "SML", "SMM", "SMP",
    "SMR", "SNAP", "SNPS", "SO", "SOFI", "SON", "SPG", "SPGI", "SPLK", "SPOT", "SPWR", "SQ", "SR",
    "SRCL", "SRE", "SRPT", "SSB", "SSNC", "SSO", "STAG", "STLD", "STM", "STT", "STX", "STZ", "SU",
    "SUI", "SUM", "SUN", "SUPN", "SWK", "SWKS", "SWN", "SYF", "SYK", "SYNA", "SYY", "T", "TAP",
    "TAT", "TDC", "TDG", "TDY", "TEAM", "TECH", "TEL", "TER", "TFC", "TFX", "TGT", "THC", "THO",
    "TIF", "TIXT", "TJX", "TKR", "TMO", "TMUS", "TOL", "TOWN", "TPG", "TPR", "TRGP", "TRMB",
    "TROW", "TRP", "TRU", "TRV", "TSCO", "TSLA", "TSN", "TT", "TTWO", "TU", "TUP", "TW", "TWLO",
    "TWO", "TWTR", "TXN", "TXT", "TYL", "UA", "UAA", "UAL", "UBER", "UBS", "UDR", "UHAL", "UI",
    "UL", "ULTA", "UNH", "UNM", "UNP", "UPS", "URI", "USB", "USFD", "USO", "V", "VAC", "VAIL",
    "VAL", "VFC", "VGR", "VGT", "VICI", "VLO", "VMC", "VMW", "VNO", "VNT", "VOO", "VRSK", "VRSN",
    "VRTX", "VSH", "VST", "VTR", "VTRS", "VZ", "WAB", "WAL", "WAT", "WBA", "WBD", "WBS", "WCC",
    "WDC", "WEC", "WELL", "WEN", "WFC", "WHR", "WING", "WIX", "WK", "WLTW", "WM", "WMB", "WMT",
    "WOLF", "WOOF", "WPC", "WPM", "WRB", "WRK", "WSC", "WSM", "WSO", "WST", "WTFC", "WTM", "WTRG",
    "WU", "WW", "WY", "WYNN", "X", "XEL", "XLB", "XLC", "XLE", "XLF", "XLI", "XLK", "XLP", "XLR",
    "XLU", "XLV", "XLY", "XOM", "XPO", "XRAY", "XRX", "XYL", "Y", "YELP", "YETI", "YUM", "ZBH",
    "ZBRA", "ZD", "ZG", "ZION", "ZM", "ZS", "ZTS", "ZWS",
];

/// Returns the full list as a Vec.
pub fn darwinex_usa_equity_symbols() -> Vec<&'static str> {
    DARWINEX_USA_EQUITY_SYMBOLS.to_vec()
}

/// Fast lookup HashSet.
pub fn darwinex_usa_equity_set() -> HashSet<String> {
    DARWINEX_USA_EQUITY_SYMBOLS
        .iter()
        .map(|s| s.to_string())
        .collect()
}
