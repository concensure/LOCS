use std::path::PathBuf;

/// Compute a rough 0-100 risk score for a patch.
///
/// Higher = riskier. The score factors in:
/// - Number of files changed (>5 = high)
/// - Lines changed (>200 = high)
/// - Protected symbols touched
pub fn compute_score(files: &[PathBuf], lines_changed: usize, protected_symbols_touched: usize) -> u32 {
    let file_score = match files.len() {
        0..=2 => 0,
        3..=5 => 10,
        6..=10 => 25,
        _ => 40,
    };

    let line_score = match lines_changed {
        0..=50 => 0,
        51..=200 => 15,
        201..=500 => 30,
        _ => 45,
    };

    let symbol_score = match protected_symbols_touched {
        0 => 0,
        1 => 15,
        _ => 30,
    };

    (file_score + line_score + symbol_score).min(100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_score_zero_for_small_patch() {
        let score = compute_score(&[PathBuf::from("src/foo.rs")], 10, 0);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_risk_score_high_for_large_patch() {
        let files: Vec<PathBuf> = (0..15).map(|i| PathBuf::from(format!("src/file{}.rs", i))).collect();
        let score = compute_score(&files, 600, 2);
        assert!(score >= 80, "Expected high risk score, got {}", score);
    }

    #[test]
    fn test_risk_score_capped_at_100() {
        let files: Vec<PathBuf> = (0..50).map(|i| PathBuf::from(format!("src/file{}.rs", i))).collect();
        let score = compute_score(&files, 10000, 10);
        assert_eq!(score, 100);
    }
}
