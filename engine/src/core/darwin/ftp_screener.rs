use rusqlite::Connection;

use super::*;

// ── DARWIN FTP Screener ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinScreenResult {
    pub ticker: String,
    pub return_pct: f64,
    pub max_drawdown: f64,
    pub sharpe: f64,
    pub trading_days: i64,
    pub avg_trades_per_day: f64,
    pub symbols_traded: Vec<String>,
    pub score: f64, // composite ranking score
}

/// Scan Darwinex FTP RETURN files to find DARWINs matching criteria.
/// Reads from /mnt/bigraidz2/Darwinex_FTP/<ticker>/RETURN
pub fn scan_darwin_ftp(
    ftp_path: &str,
    min_days: i64,
    min_return: f64,
    max_drawdown: f64,
    limit: usize,
) -> Result<Vec<DarwinScreenResult>, String> {
    let ftp_dir = std::path::Path::new(ftp_path);
    if !ftp_dir.exists() {
        return Err(format!("FTP path not found: {}", ftp_path));
    }

    let mut results: Vec<DarwinScreenResult> = Vec::new();
    let entries = std::fs::read_dir(ftp_dir).map_err(|e| format!("Read dir failed: {e}"))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let ticker = entry.file_name().to_str().unwrap_or("").to_string();
        if ticker.is_empty() || ticker.starts_with('.') {
            continue;
        }

        let return_path = entry.path().join("RETURN");
        if !return_path.exists() {
            continue;
        }

        // Parse RETURN file: timestamp,experience_score,[return_values...]
        let content = match std::fs::read_to_string(&return_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < min_days as usize {
            continue;
        }

        // Extract return values from last line
        let last_line = lines.last().unwrap_or(&"");
        let parts: Vec<&str> = last_line.splitn(3, ',').collect();
        if parts.len() < 3 {
            continue;
        }

        // Parse the return array from the last line
        let return_str = parts[2].trim_start_matches('[').trim_end_matches(']');
        let last_return: f64 = return_str
            .split(',')
            .last()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(1.0);

        let total_return = (last_return - 1.0) * 100.0;
        if total_return < min_return {
            continue;
        }

        // Compute max drawdown from all return values across all lines
        let mut peak = 0.0f64;
        let mut max_dd = 0.0f64;
        let mut daily_returns: Vec<f64> = Vec::new();
        let mut prev_val = 1.0f64;

        for line in &lines {
            let lp: Vec<&str> = line.splitn(3, ',').collect();
            if lp.len() < 3 {
                continue;
            }
            let vals_str = lp[2].trim_start_matches('[').trim_end_matches(']');
            for val_str in vals_str.split(',') {
                if let Ok(val) = val_str.trim().parse::<f64>() {
                    if val > peak {
                        peak = val;
                    }
                    if peak > 0.0 {
                        let dd = (peak - val) / peak * 100.0;
                        if dd > max_dd {
                            max_dd = dd;
                        }
                    }
                    let ret = (val - prev_val) / prev_val;
                    daily_returns.push(ret);
                    prev_val = val;
                }
            }
        }

        if max_dd > max_drawdown {
            continue;
        }

        // Compute Sharpe
        let n = daily_returns.len() as f64;
        let avg = if n > 0.0 {
            daily_returns.iter().sum::<f64>() / n
        } else {
            0.0
        };
        let vol = if n > 1.0 {
            (daily_returns.iter().map(|r| (r - avg).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
        } else {
            1.0
        };
        let sharpe = if vol > 0.0 {
            avg / vol * (252.0f64).sqrt()
        } else {
            0.0
        };

        // Read POSITIONS for symbols traded
        let positions_path = entry.path().join("POSITIONS");
        let mut symbols = Vec::new();
        if positions_path.exists() {
            if let Ok(pos_content) = std::fs::read_to_string(&positions_path) {
                let mut sym_set = std::collections::HashSet::new();
                for line in pos_content.lines() {
                    // Find symbol names in ['SYMBOL', ...] patterns
                    for part in line.split("'") {
                        if part.len() >= 2
                            && part.len() <= 10
                            && part.chars().all(|c| c.is_ascii_uppercase() || c == '/')
                        {
                            sym_set.insert(part.to_string());
                        }
                    }
                }
                symbols = sym_set.into_iter().collect();
                symbols.sort();
            }
        }

        let score = sharpe * 0.4 + total_return * 0.3 - max_dd * 0.3;

        results.push(DarwinScreenResult {
            ticker,
            return_pct: total_return,
            max_drawdown: max_dd,
            sharpe,
            trading_days: lines.len() as i64,
            avg_trades_per_day: 0.0, // would need TRADES file
            symbols_traded: symbols,
            score,
        });
    }

    // Sort by score descending
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(limit);
    Ok(results)
}

/// Export symbol radar data in web-compatible radar format.
/// Reads MT5 specs from SQLite and generates the .txt report.
pub fn export_radar_txt(
    _conn: &Connection,
    cache_conn: &Connection,
    output_dir: &str,
) -> Result<String, String> {
    let specs = load_all_specs(cache_conn)?;
    let dir = std::path::Path::new(output_dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("Create dir failed: {e}"))?;

    // Write specs snapshot for radar tracking
    let timestamp = chrono::Utc::now().format("%Y.%m.%d").to_string();

    // Parse the CSV specs and write in darwinex-radar compatible format
    // BarCacheWriter stores: Symbol,SectorName,IndustryName,TradeMode,SwapLong,SwapShort,Spread,
    //   VolumeMin,VolumeMax,VolumeStep,ContractSize,TickSize,TickValue,Digits,MarginInitial,
    //   MarginMaintenance,BaseCurrency,QuoteCurrency,Description
    let lines: Vec<&str> = specs.lines().collect();

    // Categorize symbols and write per-category CSV files for radar
    let mut stocks = Vec::new();
    let mut cfd = Vec::new();
    let mut crypto = Vec::new();
    let mut futures = Vec::new();

    for line in lines.iter() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 4 {
            continue;
        }
        let symbol = fields[0].trim();
        let sector = if fields.len() > 1 {
            fields[1].trim()
        } else {
            ""
        };

        // Classify by sector/symbol pattern
        if sector.contains("Crypto")
            || symbol.ends_with("USD")
                && (symbol.starts_with("BTC")
                    || symbol.starts_with("ETH")
                    || symbol.starts_with("SOL")
                    || symbol.starts_with("DOGE")
                    || symbol.starts_with("XRP"))
        {
            crypto.push(*line);
        } else if symbol.contains('_') || symbol.starts_with("6") {
            futures.push(*line);
        } else if sector.is_empty() || sector == "Unknown" || sector == "Forex" || symbol.len() == 6
        {
            cfd.push(*line);
        } else {
            stocks.push(*line);
        }
    }

    // Write semicolon-delimited CSVs in darwinex-radar format
    // Convert comma-separated to semicolon-separated
    let to_semicolon = |lines: &[&str]| -> String {
        let mut out = String::new();
        // Write header: Symbol;TradeMode;SwapLong;SwapShort;Spread (minimum radar columns)
        out.push_str("Symbol;TradeMode;SwapLong;SwapShort;Spread;SectorName;IndustryName;VolumeMin;VolumeMax;ContractSize;MarginInitial\n");
        for line in lines {
            let f: Vec<&str> = line.split(',').collect();
            if f.len() >= 7 {
                // Reorder: Symbol(0), TradeMode(3), SwapLong(4), SwapShort(5), Spread(6), Sector(1), Industry(2), VolMin(7), VolMax(8), Contract(10), Margin(14)
                out.push_str(&format!(
                    "{};{};{};{};{};{};{};{};{};{};{}\n",
                    f[0].trim(),
                    f.get(3).unwrap_or(&"").trim(),
                    f.get(4).unwrap_or(&"").trim(),
                    f.get(5).unwrap_or(&"").trim(),
                    f.get(6).unwrap_or(&"").trim(),
                    f.get(1).unwrap_or(&"").trim(),
                    f.get(2).unwrap_or(&"").trim(),
                    f.get(7).unwrap_or(&"").trim(),
                    f.get(8).unwrap_or(&"").trim(),
                    f.get(10).unwrap_or(&"").trim(),
                    f.get(14).unwrap_or(&"").trim(),
                ));
            }
        }
        out
    };

    let mut exported = Vec::new();
    if !stocks.is_empty() {
        let path = dir.join(format!(
            "SymbolsExport-Darwinex-Live-Stocks-{}.csv",
            timestamp
        ));
        std::fs::write(&path, to_semicolon(&stocks))
            .map_err(|e| format!("Write stocks failed: {e}"))?;
        exported.push(format!("stocks:{}", stocks.len()));
    }
    if !cfd.is_empty() {
        let path = dir.join(format!("SymbolsExport-Darwinex-Live-CFD-{}.csv", timestamp));
        std::fs::write(&path, to_semicolon(&cfd)).map_err(|e| format!("Write CFD failed: {e}"))?;
        exported.push(format!("cfd:{}", cfd.len()));
    }
    if !crypto.is_empty() {
        let path = dir.join(format!(
            "SymbolsExport-Darwinex-Live-Crypto-{}.csv",
            timestamp
        ));
        std::fs::write(&path, to_semicolon(&crypto))
            .map_err(|e| format!("Write crypto failed: {e}"))?;
        exported.push(format!("crypto:{}", crypto.len()));
    }
    if !futures.is_empty() {
        let path = dir.join(format!(
            "SymbolsExport-Darwinex-Live-Futures-{}.csv",
            timestamp
        ));
        std::fs::write(&path, to_semicolon(&futures))
            .map_err(|e| format!("Write futures failed: {e}"))?;
        exported.push(format!("futures:{}", futures.len()));
    }

    // Also write raw specs for debugging
    let raw_path = dir.join(format!("SymbolsExport-Darwinex-Live-All-{}.csv", timestamp));
    std::fs::write(&raw_path, &specs).map_err(|e| format!("Write raw failed: {e}"))?;

    // Export to optional darwinex-radar directory (web-compatible snapshots)
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let mw_dir_path = std::path::PathBuf::from(home).join("git/typhoon-darwinex-radar");
    let mw_dir = mw_dir_path.as_path();
    if mw_dir.exists() {
        let mw_timestamp = chrono::Utc::now().format("%Y.%m.%d").to_string();
        if !stocks.is_empty() {
            let _ = std::fs::write(
                mw_dir.join(format!(
                    "SymbolsExport-Darwinex-Live-Stocks-{}.csv",
                    mw_timestamp
                )),
                to_semicolon(&stocks),
            );
        }
        if !cfd.is_empty() {
            let _ = std::fs::write(
                mw_dir.join(format!(
                    "SymbolsExport-Darwinex-Live-CFD-{}.csv",
                    mw_timestamp
                )),
                to_semicolon(&cfd),
            );
        }
        if !crypto.is_empty() {
            let _ = std::fs::write(
                mw_dir.join(format!(
                    "SymbolsExport-Darwinex-Live-Crypto-{}.csv",
                    mw_timestamp
                )),
                to_semicolon(&crypto),
            );
        }
        if !futures.is_empty() {
            let _ = std::fs::write(
                mw_dir.join(format!(
                    "SymbolsExport-Darwinex-Live-Futures-{}.csv",
                    mw_timestamp
                )),
                to_semicolon(&futures),
            );
        }
        exported.push(format!("darwinex-radar:{}", mw_timestamp));
    }

    Ok(format!(
        "Exported {} ({} total symbols)",
        exported.join(", "),
        lines.len()
    ))
}

/// Radar changelog entry — one change between snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarChange {
    pub symbol: String,
    pub change_type: String, // "NEW", "REMOVED", "SWAP_CHANGED", "SPREAD_CHANGED", "MODE_CHANGED"
    pub detail: String,
}

/// Compare current specs against previous snapshot stored in KV, return changelog.
/// Also stores current snapshot for next comparison.
pub fn radar_changelog(cache_conn: &Connection) -> Result<Vec<RadarChange>, String> {
    let specs = load_all_specs(cache_conn)?;

    // Parse current specs into map: symbol -> (trade_mode, swap_long, swap_short, spread, sector, description)
    let parse_specs =
        |data: &str| -> std::collections::HashMap<String, (i32, f64, f64, i32, String, String)> {
            let mut map = std::collections::HashMap::new();
            for line in data.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let f: Vec<&str> = line.split(',').collect();
                if f.len() < 7 {
                    continue;
                }
                let sym = f[0].trim();
                if sym.is_empty() || sym == "Symbol" {
                    continue;
                }
                let mode: i32 = f.get(3).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                let swap_l: f64 = f.get(4).and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
                let swap_s: f64 = f.get(5).and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
                let spread: i32 = f.get(6).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                let sector = f.get(1).unwrap_or(&"").trim().to_string();
                let desc = f.get(18).unwrap_or(&"").trim().to_string();
                map.insert(
                    sym.to_string(),
                    (mode, swap_l, swap_s, spread, sector, desc),
                );
            }
            map
        };

    let current = parse_specs(&specs);

    // Load previous snapshot from KV
    let prev_data = {
        let sql = "SELECT value FROM kv_cache WHERE key = 'radar:previous_snapshot' LIMIT 1";
        cache_conn
            .prepare(sql)
            .ok()
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, Vec<u8>>(0)).ok())
            .and_then(|data| {
                zstd::decode_all(data.as_slice())
                    .ok()
                    .and_then(|d| String::from_utf8(d).ok())
                    .or_else(|| String::from_utf8(data).ok())
            })
    };

    let previous = prev_data.as_deref().map(parse_specs).unwrap_or_default();

    let mut changes = Vec::new();

    // New symbols
    for (sym, (mode, swap_l, swap_s, _spread, sector, desc)) in &current {
        if !previous.contains_key(sym) {
            let mode_str = match *mode {
                0 => "Disabled",
                3 => "Close-Only",
                4 => "Full",
                _ => "Partial",
            };
            changes.push(RadarChange {
                symbol: sym.clone(),
                change_type: "NEW".into(),
                detail: format!(
                    "{} [{}] SwapL:{:.2} SwapS:{:.2} {} — {}",
                    sym, mode_str, swap_l, swap_s, sector, desc
                ),
            });
        }
    }

    // Removed symbols
    for (sym, (_mode, _swap_l, _swap_s, _spread, _sector, desc)) in &previous {
        if !current.contains_key(sym) {
            changes.push(RadarChange {
                symbol: sym.clone(),
                change_type: "REMOVED".into(),
                detail: format!("{} — {}", sym, desc),
            });
        }
    }

    // Changed symbols
    for (sym, (mode, swap_l, swap_s, spread, _sector, _desc)) in &current {
        if let Some((prev_mode, prev_swap_l, prev_swap_s, prev_spread, _, _)) = previous.get(sym) {
            if mode != prev_mode {
                let mode_str = |m: i32| match m {
                    0 => "Disabled",
                    3 => "Close-Only",
                    4 => "Full",
                    _ => "Partial",
                };
                changes.push(RadarChange {
                    symbol: sym.clone(),
                    change_type: "MODE_CHANGED".into(),
                    detail: format!(
                        "{} → {} (was {})",
                        sym,
                        mode_str(*mode),
                        mode_str(*prev_mode)
                    ),
                });
            }
            if (swap_l - prev_swap_l).abs() > 0.01 || (swap_s - prev_swap_s).abs() > 0.01 {
                changes.push(RadarChange {
                    symbol: sym.clone(),
                    change_type: "SWAP_CHANGED".into(),
                    detail: format!(
                        "{} SwapL:{:.2}→{:.2} SwapS:{:.2}→{:.2}",
                        sym, prev_swap_l, swap_l, prev_swap_s, swap_s
                    ),
                });
            }
            if spread != prev_spread {
                changes.push(RadarChange {
                    symbol: sym.clone(),
                    change_type: "SPREAD_CHANGED".into(),
                    detail: format!("{} Spread:{}→{}", sym, prev_spread, spread),
                });
            }
        }
    }

    // Sort: NEW first, then REMOVED, then MODE, then SWAP, then SPREAD
    changes.sort_by(|a, b| {
        let order = |t: &str| match t {
            "NEW" => 0,
            "REMOVED" => 1,
            "MODE_CHANGED" => 2,
            "SWAP_CHANGED" => 3,
            _ => 4,
        };
        order(&a.change_type)
            .cmp(&order(&b.change_type))
            .then(a.symbol.cmp(&b.symbol))
    });

    // Store current snapshot for next comparison (compress with zstd)
    if let Ok(compressed) = zstd::encode_all(specs.as_bytes(), 3) {
        let sql = "INSERT OR REPLACE INTO kv_cache (key, value, timestamp) VALUES (?1, ?2, ?3)";
        if let Ok(mut stmt) = cache_conn.prepare(sql) {
            let _ = stmt.execute(rusqlite::params![
                "radar:previous_snapshot",
                compressed,
                chrono::Utc::now().timestamp()
            ]);
        }
    }

    Ok(changes)
}
