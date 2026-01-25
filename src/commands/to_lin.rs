use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use pbn_to_pdf::model::{Auction, Board, Call, Direction, Hand, PlaySequence, Suit, Vulnerability};
use pbn_to_pdf::parser::parse_pbn;
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    /// Input PBN file
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output LIN file (defaults to <input>.lin)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<()> {
    // Read and parse input file
    let content = std::fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read input file: {}", args.input.display()))?;

    let pbn_file =
        parse_pbn(&content).map_err(|e| anyhow::anyhow!("Failed to parse PBN: {:?}", e))?;

    // Convert each board to LIN format
    let lin_lines: Vec<String> = pbn_file.boards.iter().map(encode_board_to_lin).collect();

    let lin_content = lin_lines.join("\n");

    // Determine output path
    let output_path = args
        .output
        .unwrap_or_else(|| args.input.with_extension("lin"));

    // Write output
    std::fs::write(&output_path, &lin_content)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    println!("Converted {} boards to LIN format", pbn_file.boards.len());
    println!("Wrote to {}", output_path.display());

    Ok(())
}

/// Encode a single board to LIN format
fn encode_board_to_lin(board: &Board) -> String {
    let mut parts = Vec::new();

    // Player names (pn) - S,W,N,E order
    let south = board.players.south.as_deref().unwrap_or("");
    let west = board.players.west.as_deref().unwrap_or("");
    let north = board.players.north.as_deref().unwrap_or("");
    let east = board.players.east.as_deref().unwrap_or("");
    parts.push(format!("pn|{},{},{},{}|", south, west, north, east));

    // Make deal (md) - dealer digit + hands in S,W,N order
    let dealer = board.dealer.unwrap_or(Direction::North);
    let dealer_digit = match dealer {
        Direction::South => '1',
        Direction::West => '2',
        Direction::North => '3',
        Direction::East => '4',
    };

    let south_hand = encode_hand(&board.deal.south);
    let west_hand = encode_hand(&board.deal.west);
    let north_hand = encode_hand(&board.deal.north);
    // East hand is implied (not included in LIN)

    parts.push(format!(
        "md|{}{},{},{}|",
        dealer_digit, south_hand, west_hand, north_hand
    ));

    // Vulnerability (sv)
    let sv = match board.vulnerable {
        Vulnerability::None => "o",
        Vulnerability::NorthSouth => "n",
        Vulnerability::EastWest => "e",
        Vulnerability::Both => "b",
    };
    parts.push(format!("sv|{}|", sv));

    // Board header (ah)
    if let Some(num) = board.number {
        parts.push(format!("ah|Board {}|", num));
    }

    // Auction (mb)
    if let Some(ref auction) = board.auction {
        encode_auction(&mut parts, auction);
    }

    // Play sequence (pc)
    if let Some(ref play) = board.play {
        encode_play(&mut parts, play);
    }

    parts.join("")
}

/// Encode a hand to LIN format (SHDC order)
fn encode_hand(hand: &Hand) -> String {
    let mut result = String::new();

    // Spades
    if !hand.spades.is_empty() {
        result.push('S');
        for rank in &hand.spades.ranks {
            result.push(rank.to_char());
        }
    }

    // Hearts
    if !hand.hearts.is_empty() {
        result.push('H');
        for rank in &hand.hearts.ranks {
            result.push(rank.to_char());
        }
    }

    // Diamonds
    if !hand.diamonds.is_empty() {
        result.push('D');
        for rank in &hand.diamonds.ranks {
            result.push(rank.to_char());
        }
    }

    // Clubs
    if !hand.clubs.is_empty() {
        result.push('C');
        for rank in &hand.clubs.ranks {
            result.push(rank.to_char());
        }
    }

    result
}

/// Encode auction to LIN format
fn encode_auction(parts: &mut Vec<String>, auction: &Auction) {
    for annotated_call in &auction.calls {
        let bid_str = match &annotated_call.call {
            Call::Pass => "p".to_string(),
            Call::Double => "d".to_string(),
            Call::Redouble => "r".to_string(),
            Call::Continue => continue, // Skip continue markers in LIN
            Call::Bid { level, strain } => {
                let strain_str = strain.to_char();
                format!("{}{}", level, strain_str)
            }
        };

        // Add alert marker if annotation present
        let has_annotation = annotated_call.annotation.is_some();
        let alert_marker = if has_annotation { "!" } else { "" };
        parts.push(format!("mb|{}{}|", bid_str, alert_marker));

        // Add annotation if present
        if let Some(ref annotation) = annotated_call.annotation {
            // Replace spaces with + for LIN format
            let encoded_note = annotation.replace(' ', "+");
            parts.push(format!("an|{}|", encoded_note));
        }
    }
}

/// Encode play sequence to LIN format
fn encode_play(parts: &mut Vec<String>, play: &PlaySequence) {
    for trick in &play.tricks {
        for card in trick.cards.iter().flatten() {
            let suit_char = match card.suit {
                Suit::Spades => 'S',
                Suit::Hearts => 'H',
                Suit::Diamonds => 'D',
                Suit::Clubs => 'C',
            };
            parts.push(format!("pc|{}{}|", suit_char, card.rank.to_char()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pbn_to_pdf::model::{Holding, Rank};

    #[test]
    fn test_encode_hand() {
        let mut hand = Hand::new();
        hand.spades = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]);
        hand.hearts = Holding::from_ranks([Rank::Jack, Rank::Ten, Rank::Nine]);
        hand.diamonds = Holding::from_ranks([Rank::Eight, Rank::Seven, Rank::Six]);
        hand.clubs = Holding::from_ranks([Rank::Five, Rank::Four, Rank::Three, Rank::Two]);

        let encoded = encode_hand(&hand);
        assert_eq!(encoded, "SAKQHJT9D876C5432");
    }

    #[test]
    fn test_encode_hand_with_voids() {
        let mut hand = Hand::new();
        hand.spades = Holding::from_ranks([
            Rank::Ace,
            Rank::King,
            Rank::Queen,
            Rank::Jack,
            Rank::Ten,
            Rank::Nine,
            Rank::Eight,
        ]);
        // Hearts void
        hand.diamonds = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]);
        hand.clubs = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]);

        let encoded = encode_hand(&hand);
        assert_eq!(encoded, "SAKQJT98DAKQCAKQ");
    }

    #[test]
    fn test_vulnerability_encoding() {
        assert_eq!(
            match Vulnerability::None {
                Vulnerability::None => "o",
                Vulnerability::NorthSouth => "n",
                Vulnerability::EastWest => "e",
                Vulnerability::Both => "b",
            },
            "o"
        );
    }
}
