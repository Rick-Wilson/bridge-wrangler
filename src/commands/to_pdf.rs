use anyhow::{Context, Result};
use clap::{Args as ClapArgs, ValueEnum};
use pbn_to_pdf::cli::Layout as PdfLayout;
use pbn_to_pdf::{
    config::Settings,
    parser::parse_pbn,
    render::{generate_pdf, BiddingSheetsRenderer, DeclarersPlanRenderer},
};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum Layout {
    /// Full hand diagram with bidding table and commentary
    #[default]
    Analysis,
    /// Simplified layout for practice bidding
    BiddingSheets,
    /// 4 deals per page for declarer's planning practice
    DeclarersPlan,
}

impl From<Layout> for PdfLayout {
    fn from(layout: Layout) -> Self {
        match layout {
            Layout::Analysis => PdfLayout::Analysis,
            Layout::BiddingSheets => PdfLayout::BiddingSheets,
            Layout::DeclarersPlan => PdfLayout::DeclarersPlan,
        }
    }
}

#[derive(ClapArgs)]
pub struct Args {
    /// Input PBN file
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output PDF file (defaults to input with .pdf extension)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Layout style
    #[arg(short, long, value_enum, default_value = "analysis")]
    pub layout: Layout,

    /// Boards per page (1, 2, or 4)
    #[arg(short, long)]
    pub boards_per_page: Option<u8>,

    /// Board range to include (e.g., "1-4" or "1,3,5")
    #[arg(short = 'r', long)]
    pub board_range: Option<String>,

    /// Hide bidding
    #[arg(long)]
    pub hide_bidding: bool,

    /// Hide play sequence
    #[arg(long)]
    pub hide_play: bool,

    /// Hide commentary
    #[arg(long)]
    pub hide_commentary: bool,

    /// Show high card points
    #[arg(long)]
    pub show_hcp: bool,
}

pub fn run(args: Args) -> Result<()> {
    // Read input file
    let content = std::fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read input file: {}", args.input.display()))?;

    // Parse PBN
    let pbn_file =
        parse_pbn(&content).map_err(|e| anyhow::anyhow!("Failed to parse PBN: {:?}", e))?;

    println!(
        "Parsed {} boards from {}",
        pbn_file.boards.len(),
        args.input.display()
    );

    // Filter boards if range specified
    let boards = if let Some(ref range) = args.board_range {
        let allowed = parse_board_range(range)?;
        pbn_file
            .boards
            .into_iter()
            .filter(|b| b.number.map(|n| allowed.contains(&n)).unwrap_or(false))
            .collect::<Vec<_>>()
    } else {
        pbn_file.boards
    };

    if boards.is_empty() {
        return Err(anyhow::anyhow!("No boards to render after filtering"));
    }

    // Build settings
    let mut settings = Settings::default().with_metadata(&pbn_file.metadata);

    // Apply CLI overrides
    settings.layout = args.layout.into();

    if let Some(bpp) = args.boards_per_page {
        settings.boards_per_page = bpp;
    }

    if args.hide_bidding {
        settings.show_bidding = false;
    }
    if args.hide_play {
        settings.show_play = false;
    }
    if args.hide_commentary {
        settings.show_commentary = false;
    }
    if args.show_hcp {
        settings.show_hcp = true;
    }

    // Generate PDF using the appropriate renderer for the layout
    let pdf_bytes = match settings.layout {
        PdfLayout::Analysis => generate_pdf(&boards, &settings)
            .map_err(|e| anyhow::anyhow!("Failed to generate PDF: {:?}", e))?,
        PdfLayout::BiddingSheets => {
            let renderer = BiddingSheetsRenderer::new(settings.clone());
            renderer
                .render(&boards)
                .map_err(|e| anyhow::anyhow!("Failed to generate bidding sheets PDF: {:?}", e))?
        }
        PdfLayout::DeclarersPlan => {
            let renderer = DeclarersPlanRenderer::new(settings.clone());
            renderer
                .render(&boards)
                .map_err(|e| anyhow::anyhow!("Failed to generate declarer's plan PDF: {:?}", e))?
        }
        _ => generate_pdf(&boards, &settings)
            .map_err(|e| anyhow::anyhow!("Failed to generate PDF: {:?}", e))?,
    };

    // Determine output path
    let output_path = args
        .output
        .unwrap_or_else(|| args.input.with_extension("pdf"));

    // Write output
    std::fs::write(&output_path, &pdf_bytes)
        .with_context(|| format!("Failed to write PDF: {}", output_path.display()))?;

    println!("Wrote {} boards to {}", boards.len(), output_path.display());

    Ok(())
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
}
