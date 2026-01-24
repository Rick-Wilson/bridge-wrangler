use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use regex::Regex;
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    /// Input PBN file
    #[arg(short, long)]
    pub input: PathBuf,

    /// Regex pattern to match against each board
    #[arg(short = 'p', long)]
    pub pattern: String,

    /// Output file for matched boards
    #[arg(short = 'm', long)]
    pub matched: Option<PathBuf>,

    /// Output file for non-matched boards
    #[arg(short = 'n', long)]
    pub not_matched: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<()> {
    // Compile the regex pattern
    let re = Regex::new(&args.pattern)
        .with_context(|| format!("Invalid regex pattern: {}", args.pattern))?;

    // Read input file
    let content = std::fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read input file: {}", args.input.display()))?;

    // Split content into header and board sections
    let (header, board_sections) = split_pbn_content(&content);

    let total_boards = board_sections.len();

    if total_boards == 0 {
        println!("No boards found in input file");
        return Ok(());
    }

    // Filter boards
    let mut matched_boards = Vec::new();
    let mut not_matched_boards = Vec::new();

    for section in &board_sections {
        if re.is_match(section) {
            matched_boards.push(section.clone());
        } else {
            not_matched_boards.push(section.clone());
        }
    }

    let matched_count = matched_boards.len();
    let not_matched_count = not_matched_boards.len();
    let match_percent = (matched_count as f64 / total_boards as f64) * 100.0;

    // Write matched boards if output specified
    if let Some(ref matched_path) = args.matched {
        let output = build_output(&header, &matched_boards);
        std::fs::write(matched_path, output)
            .with_context(|| format!("Failed to write matched file: {}", matched_path.display()))?;
        println!("Wrote {} matched boards to {}", matched_count, matched_path.display());
    }

    // Write non-matched boards if output specified
    if let Some(ref not_matched_path) = args.not_matched {
        let output = build_output(&header, &not_matched_boards);
        std::fs::write(not_matched_path, output)
            .with_context(|| format!("Failed to write not-matched file: {}", not_matched_path.display()))?;
        println!("Wrote {} non-matched boards to {}", not_matched_count, not_matched_path.display());
    }

    // Report results
    println!();
    println!("Filter results for pattern: {}", args.pattern);
    println!("  Boards searched:    {}", total_boards);
    println!("  Boards matched:     {}", matched_count);
    println!("  Boards not matched: {}", not_matched_count);
    println!("  Match rate:         {:.1}%", match_percent);

    Ok(())
}

/// Build output content from header and board sections
fn build_output(header: &str, boards: &[String]) -> String {
    let mut output = String::new();
    output.push_str(header);
    for board in boards {
        output.push_str(board);
        if !board.ends_with('\n') {
            output.push('\n');
        }
    }
    output
}

/// Split PBN content into header and individual board sections
fn split_pbn_content(content: &str) -> (String, Vec<String>) {
    let mut header = String::new();
    let mut board_sections: Vec<String> = Vec::new();
    let mut current_board = String::new();
    let mut in_header = true;

    for line in content.lines() {
        let trimmed = line.trim();

        if in_header {
            if trimmed.starts_with('%') || trimmed.is_empty() {
                header.push_str(line);
                header.push('\n');
            } else if trimmed.starts_with('[') {
                in_header = false;
                current_board.push_str(line);
                current_board.push('\n');
            }
        } else {
            // Check if this is the start of a new board (Event tag typically starts a board)
            if trimmed.starts_with("[Event ") && !current_board.is_empty() {
                board_sections.push(std::mem::take(&mut current_board));
            }
            current_board.push_str(line);
            current_board.push('\n');
        }
    }

    // Don't forget the last board
    if !current_board.is_empty() {
        board_sections.push(current_board);
    }

    (header, board_sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_matching() {
        let re = Regex::new(r#"\[Contract "3NT"\]"#).unwrap();
        let board = r#"[Board "1"]
[Contract "3NT"]
[Deal "N:..."]
"#;
        assert!(re.is_match(board));
    }

    #[test]
    fn test_split_pbn_content() {
        let content = r#"% PBN 2.1
%Creator: Test
[Event "Test"]
[Board "1"]
[Deal "N:..."]

[Event "Test"]
[Board "2"]
[Deal "N:..."]
"#;
        let (header, boards) = split_pbn_content(content);
        assert!(header.contains("% PBN 2.1"));
        assert_eq!(boards.len(), 2);
    }
}
