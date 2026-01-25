use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use regex::Regex;
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    /// Input PBN file
    #[arg(short, long)]
    pub input: PathBuf,

    /// Event name to set
    #[arg(short, long)]
    pub event: String,

    /// Output file (defaults to <input>-Updated.pbn)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Update the input file in place
    #[arg(long, conflicts_with = "output")]
    pub in_place: bool,
}

pub fn run(args: Args) -> Result<()> {
    // Read input file
    let content = std::fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read input file: {}", args.input.display()))?;

    // Update all Event tags
    let event_re = Regex::new(r#"\[Event\s+"[^"]*"\]"#).unwrap();
    let new_event_tag = format!("[Event \"{}\"]", args.event);
    let updated_content = event_re.replace_all(&content, new_event_tag.as_str());

    // Count how many replacements were made
    let match_count = event_re.find_iter(&content).count();

    // Determine output path
    let output_path = if args.in_place {
        args.input.clone()
    } else {
        args.output.unwrap_or_else(|| {
            let stem = args.input.file_stem().unwrap_or_default().to_string_lossy();
            let parent = args.input.parent().unwrap_or(std::path::Path::new("."));
            parent.join(format!("{}-Updated.pbn", stem))
        })
    };

    // Write output
    std::fs::write(&output_path, updated_content.as_ref())
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    // Report results
    if args.in_place {
        println!(
            "Updated {} Event tags in {} to \"{}\"",
            match_count,
            args.input.display(),
            args.event
        );
    } else {
        println!(
            "Updated {} Event tags, wrote to {}",
            match_count,
            output_path.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_regex() {
        let re = Regex::new(r#"\[Event\s+"[^"]*"\]"#).unwrap();
        let content = r#"[Event "Old Event"]
[Board "1"]
[Event "Another"]"#;
        let new_tag = "[Event \"New Event\"]";
        let result = re.replace_all(content, new_tag);
        assert!(result.contains("[Event \"New Event\"]"));
        assert!(!result.contains("Old Event"));
        assert!(!result.contains("Another"));
    }
}
