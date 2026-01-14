mod font_builder;
mod svg_parser;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "svg2font")]
#[command(about = "Convert SVG icons to TTF icon font")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate TTF font from SVG icons
    Generate {
        /// Input directory containing SVG files
        #[arg(short, long, default_value = "./icons")]
        input: PathBuf,

        /// Output directory for generated files
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,

        /// Font family name
        #[arg(short, long, default_value = "Icons")]
        name: String,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            input,
            output,
            name,
            verbose,
        } => {
            generate_font(&input, &output, &name, verbose)?;
        }
    }

    Ok(())
}

fn generate_font(input: &Path, output: &Path, font_name: &str, verbose: bool) -> Result<()> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output)?;

    if verbose {
        println!("Scanning SVG files in: {}", input.display());
    }

    // Parse all SVG files
    let icons = svg_parser::parse_svg_directory(input, verbose)?;

    if icons.is_empty() {
        anyhow::bail!("No SVG files found in {}", input.display());
    }

    println!("Found {} icons", icons.len());

    // Build the font
    let ttf_path = output.join(format!(
        "{}.ttf",
        font_name.to_lowercase().replace(' ', "_")
    ));
    font_builder::build_font(&icons, font_name, &ttf_path, verbose)?;

    println!("Generated: {}", ttf_path.display());
    println!("\nDone! {} icons processed.", icons.len());

    Ok(())
}
