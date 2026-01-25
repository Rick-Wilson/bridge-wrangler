use anyhow::{Context, Result};
use bridge_parsers::{Board, Direction, Vulnerability};
use bridge_parsers::pbn::read_pbn;
use bridge_solver::{
    CutoffCache, Hands, PatternCache, Solver, CLUB, DIAMOND, EAST, HEART, NORTH, NOTRUMP, SOUTH,
    SPADE, WEST,
};
use clap::Args as ClapArgs;
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    /// Input PBN file
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output PBN file with DD results (optional)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Board range to analyze (e.g., "1-4" or "1,3,5")
    #[arg(short = 'r', long)]
    pub board_range: Option<String>,

    /// Show detailed output for each board
    #[arg(short, long)]
    pub verbose: bool,
}

/// DD analysis results for a single board
#[derive(Debug, Clone)]
pub struct DdResults {
    /// Tricks by declarer (N, S, E, W) and denomination (NT, S, H, D, C)
    /// results[declarer_idx][denom_idx] = tricks
    pub tricks: [[u8; 5]; 4],
}

impl DdResults {
    /// Format as OptimumResultTable PBN tag value
    pub fn to_optimum_result_table(&self) -> String {
        // Format: "NT\tS\tH\tD\tC\nN\t7\t8\t6\t7\t9\nS\t7\t8\t6\t7\t9\nE\t6\t5\t7\t6\t4\nW\t6\t5\t7\t6\t4"
        let mut lines = vec!["NT\tS\tH\tD\tC".to_string()];
        let seats = ['N', 'S', 'E', 'W'];
        for (i, seat) in seats.iter().enumerate() {
            let row: Vec<String> = self.tricks[i].iter().map(|t| t.to_string()).collect();
            lines.push(format!("{}\t{}", seat, row.join("\t")));
        }
        lines.join("\\n")
    }

    /// Format as human-readable table
    pub fn to_display_table(&self) -> String {
        let mut output = String::new();
        output.push_str("       NT   S   H   D   C\n");
        let seats = ["N", "S", "E", "W"];
        for (i, seat) in seats.iter().enumerate() {
            output.push_str(&format!(
                "  {}    {:2}  {:2}  {:2}  {:2}  {:2}\n",
                seat,
                self.tricks[i][0],
                self.tricks[i][1],
                self.tricks[i][2],
                self.tricks[i][3],
                self.tricks[i][4]
            ));
        }
        output
    }

    /// Get the par contract(s) and score
    pub fn par_score(&self, vul_ns: bool, vul_ew: bool) -> (String, i32) {
        // Find the best contract for each side
        let (ns_contract, ns_score) = self.best_contract_for_side(true, vul_ns, vul_ew);
        let (ew_contract, ew_score) = self.best_contract_for_side(false, vul_ew, vul_ns);

        // The par is the result after competitive bidding
        // If NS can make game and EW can't profitably sacrifice, NS plays game
        // This is a simplified par calculation
        if ns_score >= -ew_score {
            (ns_contract, ns_score)
        } else {
            (ew_contract, -ew_score)
        }
    }

    fn best_contract_for_side(
        &self,
        is_ns: bool,
        declarer_vul: bool,
        _defender_vul: bool,
    ) -> (String, i32) {
        let declarers: &[usize] = if is_ns { &[0, 1] } else { &[2, 3] };
        let denoms = ["NT", "S", "H", "D", "C"];
        let seats = ["N", "S", "E", "W"];

        let mut best_contract = String::new();
        let mut best_score = i32::MIN;

        for &decl in declarers {
            for (denom_idx, denom) in denoms.iter().enumerate() {
                let tricks = self.tricks[decl][denom_idx];
                // Try different contract levels
                for level in 1..=7 {
                    let required = level + 6;
                    if tricks >= required {
                        let score = calculate_score(level, denom_idx, tricks, declarer_vul, false);
                        if score > best_score {
                            best_score = score;
                            best_contract = format!("{}{} by {}", level, denom, seats[decl]);
                        }
                    }
                }
            }
        }

        if best_contract.is_empty() {
            best_contract = "Pass".to_string();
            best_score = 0;
        }

        (best_contract, best_score)
    }
}

/// Calculate the score for a made contract
fn calculate_score(level: u8, denom_idx: usize, tricks: u8, vul: bool, doubled: bool) -> i32 {
    let overtricks = tricks as i32 - (level as i32 + 6);

    // Trick values
    let trick_value = match denom_idx {
        0 => 30, // NT (but first trick is 40)
        1 | 2 => 30, // Major
        _ => 20, // Minor
    };

    let mut score = if denom_idx == 0 {
        40 + (level as i32 - 1) * 30 // NT: 40 for first, 30 for rest
    } else {
        level as i32 * trick_value
    };

    if doubled {
        score *= 2;
    }

    // Game/slam bonuses
    let game_threshold = match denom_idx {
        0 => 3,    // 3NT
        1 | 2 => 4, // 4M
        _ => 5,    // 5m
    };

    if level >= game_threshold {
        score += if vul { 500 } else { 300 }; // Game bonus
    } else {
        score += 50; // Part score bonus
    }

    if level == 6 {
        score += if vul { 750 } else { 500 }; // Small slam
    } else if level == 7 {
        score += if vul { 1500 } else { 1000 }; // Grand slam
    }

    // Overtricks
    let overtrick_value = if doubled {
        if vul { 200 } else { 100 }
    } else {
        trick_value
    };
    score += overtricks * overtrick_value;

    score
}

pub fn run(args: Args) -> Result<()> {
    // Read and parse PBN file
    let content = std::fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read input file: {}", args.input.display()))?;

    let boards = read_pbn(&content).map_err(|e| anyhow::anyhow!("Failed to parse PBN: {:?}", e))?;

    println!(
        "Read {} boards from {}",
        boards.len(),
        args.input.display()
    );

    // Filter boards if range specified
    let boards: Vec<Board> = if let Some(ref range) = args.board_range {
        let allowed = parse_board_range(range)?;
        boards
            .into_iter()
            .filter(|b| b.number.map(|n| allowed.contains(&n)).unwrap_or(false))
            .collect()
    } else {
        boards
    };

    if boards.is_empty() {
        return Err(anyhow::anyhow!("No boards to analyze after filtering"));
    }

    // Analyze each board
    let mut results: Vec<(u32, DdResults)> = Vec::new();

    for board in &boards {
        let board_num = board.number.unwrap_or(0);

        // Convert deal to solver format
        let hands = match board_to_hands(board) {
            Some(h) => h,
            None => {
                println!("Board {}: No deal found, skipping", board_num);
                continue;
            }
        };

        println!("Analyzing board {}...", board_num);

        let dd_results = analyze_deal(&hands);

        if args.verbose {
            println!("Board {}:", board_num);
            println!("{}", dd_results.to_display_table());

            // Show par if vulnerability is known
            let (vul_ns, vul_ew) = match board.vulnerable {
                Vulnerability::None => (false, false),
                Vulnerability::NorthSouth => (true, false),
                Vulnerability::EastWest => (false, true),
                Vulnerability::Both => (true, true),
            };
            let (par_contract, par_score) = dd_results.par_score(vul_ns, vul_ew);
            println!("  Par: {} ({})\n", par_contract, par_score);
        }

        results.push((board_num, dd_results));
    }

    println!("Analyzed {} boards", results.len());

    // Write output PBN with DD tags if requested
    if let Some(output_path) = args.output {
        let output_content = add_dd_tags_to_pbn(&content, &results)?;
        std::fs::write(&output_path, output_content)
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
        println!("\nWrote PBN with DD results to {}", output_path.display());
    }

    Ok(())
}

/// Convert a Board's deal to solver Hands format
fn board_to_hands(board: &Board) -> Option<Hands> {
    let deal = &board.deal;

    // Check if deal has cards (at least one hand has cards)
    if deal.hand(Direction::North).len() == 0
        && deal.hand(Direction::East).len() == 0
        && deal.hand(Direction::South).len() == 0
        && deal.hand(Direction::West).len() == 0
    {
        return None;
    }

    // Build PBN deal string: "N:spades.hearts.diamonds.clubs spades.hearts.diamonds.clubs ..."
    // Order is N E S W
    let pbn_deal = format!(
        "N:{} {} {} {}",
        deal.hand(Direction::North).to_pbn(),
        deal.hand(Direction::East).to_pbn(),
        deal.hand(Direction::South).to_pbn(),
        deal.hand(Direction::West).to_pbn()
    );

    Hands::from_pbn(&pbn_deal)
}

/// Perform DD analysis on a deal
fn analyze_deal(hands: &Hands) -> DdResults {
    let declarers = [NORTH, SOUTH, EAST, WEST];
    let denominations = [NOTRUMP, SPADE, HEART, DIAMOND, CLUB];
    let mut results = [[0u8; 5]; 4];

    for (denom_idx, &trump) in denominations.iter().enumerate() {
        // Create caches once per denomination for efficiency
        let mut cutoff_cache = CutoffCache::new(16);
        let mut pattern_cache = PatternCache::new(16);

        for (decl_idx, &declarer_seat) in declarers.iter().enumerate() {
            // Leader is to the left of declarer
            let leader = (declarer_seat + 1) % 4;

            let solver = Solver::new(*hands, trump, leader);
            let ns_tricks = solver.solve_with_caches(&mut cutoff_cache, &mut pattern_cache);

            // Convert NS tricks to declarer's tricks
            let declarer_tricks = if declarer_seat == NORTH || declarer_seat == SOUTH {
                ns_tricks
            } else {
                hands.num_tricks() as u8 - ns_tricks
            };

            results[decl_idx][denom_idx] = declarer_tricks;
        }
    }

    DdResults { tricks: results }
}

/// Add DD result tags to PBN content
fn add_dd_tags_to_pbn(content: &str, results: &[(u32, DdResults)]) -> Result<String> {
    let results_map: std::collections::HashMap<u32, &DdResults> =
        results.iter().map(|(n, r)| (*n, r)).collect();

    let mut output = String::new();
    let mut current_board: Option<u32> = None;
    let mut inserted_dd = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Track board number
        if trimmed.starts_with("[Board ") {
            // If we have a previous board and didn't insert DD, do it now
            if let Some(board_num) = current_board {
                if !inserted_dd {
                    if let Some(dd) = results_map.get(&board_num) {
                        output.push_str(&format!(
                            "[OptimumResultTable \"{}\"]\n",
                            dd.to_optimum_result_table()
                        ));
                    }
                }
            }

            // Parse new board number
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    if let Ok(num) = trimmed[start + 1..start + 1 + end].parse::<u32>() {
                        current_board = Some(num);
                        inserted_dd = false;
                    }
                }
            }
        }

        // Skip existing OptimumResultTable tags (we'll replace them)
        if trimmed.starts_with("[OptimumResultTable ") {
            inserted_dd = true; // Mark as handled
            if let Some(board_num) = current_board {
                if let Some(dd) = results_map.get(&board_num) {
                    output.push_str(&format!(
                        "[OptimumResultTable \"{}\"]\n",
                        dd.to_optimum_result_table()
                    ));
                }
            }
            continue;
        }

        // Insert DD tags before Deal tag if we haven't yet
        if trimmed.starts_with("[Deal ") && !inserted_dd {
            if let Some(board_num) = current_board {
                if let Some(dd) = results_map.get(&board_num) {
                    output.push_str(&format!(
                        "[OptimumResultTable \"{}\"]\n",
                        dd.to_optimum_result_table()
                    ));
                    inserted_dd = true;
                }
            }
        }

        output.push_str(line);
        output.push('\n');
    }

    // Handle last board
    if let Some(board_num) = current_board {
        if !inserted_dd {
            if let Some(dd) = results_map.get(&board_num) {
                output.push_str(&format!(
                    "[OptimumResultTable \"{}\"]\n",
                    dd.to_optimum_result_table()
                ));
            }
        }
    }

    Ok(output)
}

/// Parse a board range specification like "1-4" or "1,3,5" or "1-4,7,9-12"
fn parse_board_range(range: &str) -> Result<Vec<u32>> {
    let mut boards = Vec::new();

    for part in range.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid range: {}", part));
            }
            let start: u32 = parts[0]
                .trim()
                .parse()
                .with_context(|| format!("Invalid number in range: {}", parts[0]))?;
            let end: u32 = parts[1]
                .trim()
                .parse()
                .with_context(|| format!("Invalid number in range: {}", parts[1]))?;
            for i in start..=end {
                boards.push(i);
            }
        } else {
            let num: u32 = part
                .parse()
                .with_context(|| format!("Invalid board number: {}", part))?;
            boards.push(num);
        }
    }

    Ok(boards)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_board_range() {
        assert_eq!(parse_board_range("1-4").unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(parse_board_range("1,3,5").unwrap(), vec![1, 3, 5]);
        assert_eq!(parse_board_range("1-3,7").unwrap(), vec![1, 2, 3, 7]);
        assert_eq!(parse_board_range("1").unwrap(), vec![1]);
    }

    #[test]
    fn test_calculate_score() {
        // 3NT making exactly, not vul
        assert_eq!(calculate_score(3, 0, 9, false, false), 400); // 100 + 300 game

        // 4S making exactly, vul
        assert_eq!(calculate_score(4, 1, 10, true, false), 620); // 120 + 500 game

        // 3NT with 2 overtricks, not vul
        assert_eq!(calculate_score(3, 0, 11, false, false), 460); // 100 + 300 + 60
    }
}
