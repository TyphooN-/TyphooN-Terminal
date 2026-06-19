use super::{DiffChunk, SecFiling};
use rusqlite::{Connection, OptionalExtension};

// ── Filing Diff Comparison ───────────────────────────────────────────────────

/// A chunk of diff output.
pub fn diff_filing_content(old: &str, new: &str) -> Vec<DiffChunk> {
    let old_paras: Vec<&str> = old
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let new_paras: Vec<&str> = new
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // Simple LCS-based diff at paragraph level
    let m = old_paras.len();
    let n = new_paras.len();
    // Build LCS table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if old_paras[i - 1] == new_paras[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    // Backtrack to build diff
    let mut chunks = Vec::new();
    let (mut i, mut j) = (m, n);
    let mut stack = Vec::new();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_paras[i - 1] == new_paras[j - 1] {
            stack.push(DiffChunk::Same(old_paras[i - 1].to_string()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            stack.push(DiffChunk::Added(new_paras[j - 1].to_string()));
            j -= 1;
        } else if i > 0 {
            stack.push(DiffChunk::Removed(old_paras[i - 1].to_string()));
            i -= 1;
        }
    }
    stack.reverse();
    chunks.extend(stack);
    chunks
}

/// Find the previous filing of the same type for the same ticker.
pub fn find_previous_filing(
    conn: &Connection,
    ticker: &str,
    form_type: &str,
    current_date: &str,
) -> Result<Option<SecFiling>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, ticker, form_type, accession_number, filing_date, url, company_name, importance_score, category, summary, insider_flag, created_at
         FROM sec_filings WHERE ticker = ?1 AND form_type = ?2 AND filing_date < ?3
         ORDER BY filing_date DESC LIMIT 1"
    ).map_err(|e| format!("Prepare previous filing failed: {e}"))?;

    stmt.query_row(
        rusqlite::params![ticker.to_uppercase(), form_type, current_date],
        |row| {
            Ok(SecFiling {
                id: row.get(0)?,
                ticker: row.get(1)?,
                form_type: row.get(2)?,
                accession_number: row.get(3)?,
                filing_date: row.get(4)?,
                url: row.get(5)?,
                company_name: row.get(6)?,
                importance_score: row.get(7)?,
                category: row.get(8)?,
                summary: row.get(9)?,
                insider_flag: row.get(10)?,
                created_at: row.get(11)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("Query previous filing failed: {e}"))
}
