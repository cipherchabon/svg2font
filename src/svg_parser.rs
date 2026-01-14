use anyhow::{Context, Result};
use kurbo::BezPath;
use std::path::Path;
use usvg::{Options, Tree};
use walkdir::WalkDir;

/// Represents a parsed icon with its name and path data
#[derive(Debug, Clone)]
pub struct Icon {
    /// Icon name derived from filename (e.g., "arrow_down" from "arrowDown-filled.svg")
    pub name: String,
    /// Original filename without extension
    pub filename: String,
    /// Bezier path representing the icon shape
    pub path: BezPath,
    /// Original viewBox width
    pub width: f64,
    /// Original viewBox height
    pub height: f64,
    /// Unicode codepoint assigned to this icon (set later)
    pub codepoint: u32,
}

/// Parse all SVG files in a directory
pub fn parse_svg_directory(dir: &Path, verbose: bool) -> Result<Vec<Icon>> {
    let mut icons = Vec::new();
    let mut codepoint = 0xE000u32; // Start at Private Use Area

    let mut entries: Vec<_> = WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "svg")
                .unwrap_or(false)
        })
        .collect();

    // Sort for deterministic codepoint assignment
    entries.sort_by(|a, b| a.file_name().cmp(b.file_name()));

    for entry in entries {
        let path = entry.path();
        match parse_svg_file(path, codepoint) {
            Ok(icon) => {
                if verbose {
                    println!("  Parsed: {} -> U+{:04X}", icon.filename, icon.codepoint);
                }
                icons.push(icon);
                codepoint += 1;
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
            }
        }
    }

    Ok(icons)
}

/// Parse a single SVG file
fn parse_svg_file(path: &Path, codepoint: u32) -> Result<Icon> {
    let svg_content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Convert filename to valid Dart identifier
    let name = filename_to_identifier(&filename);

    let opt = Options::default();
    let tree = Tree::from_str(&svg_content, &opt)
        .with_context(|| format!("Failed to parse SVG: {}", path.display()))?;

    let size = tree.size();
    let width = size.width() as f64;
    let height = size.height() as f64;

    // Extract all paths from the SVG
    let bez_path = extract_paths(&tree);

    Ok(Icon {
        name,
        filename,
        path: bez_path,
        width,
        height,
        codepoint,
    })
}

/// Extract all paths from an SVG tree into a single BezPath
fn extract_paths(tree: &Tree) -> BezPath {
    let mut combined = BezPath::new();
    collect_paths_recursive(tree.root(), &mut combined);
    combined
}

/// Recursively collect paths from a group and its children
fn collect_paths_recursive(group: &usvg::Group, combined: &mut BezPath) {
    for node in group.children() {
        match node {
            usvg::Node::Path(ref path) => {
                let bez = usvg_path_to_kurbo(path);
                for el in bez.elements() {
                    combined.push(*el);
                }
            }
            usvg::Node::Group(ref g) => {
                collect_paths_recursive(g, combined);
            }
            _ => {}
        }
    }
}

/// Convert a usvg path to a kurbo BezPath
fn usvg_path_to_kurbo(path: &usvg::Path) -> BezPath {
    let mut bez = BezPath::new();
    let data = path.data();

    for segment in data.segments() {
        match segment {
            usvg::tiny_skia_path::PathSegment::MoveTo(pt) => {
                bez.move_to((pt.x as f64, pt.y as f64));
            }
            usvg::tiny_skia_path::PathSegment::LineTo(pt) => {
                bez.line_to((pt.x as f64, pt.y as f64));
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(pt1, pt2) => {
                bez.quad_to((pt1.x as f64, pt1.y as f64), (pt2.x as f64, pt2.y as f64));
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(pt1, pt2, pt3) => {
                bez.curve_to(
                    (pt1.x as f64, pt1.y as f64),
                    (pt2.x as f64, pt2.y as f64),
                    (pt3.x as f64, pt3.y as f64),
                );
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                bez.close_path();
            }
        }
    }

    bez
}

/// Convert a filename to a valid Dart identifier
fn filename_to_identifier(filename: &str) -> String {
    // Remove common suffixes
    let name = filename
        .replace("-filled", "Filled")
        .replace("-stroke", "Stroke")
        .replace("-outline", "Outline");

    // Convert to snake_case
    let mut result = String::new();
    let mut prev_lower = false;

    for c in name.chars() {
        if c == '-' || c == ' ' {
            result.push('_');
            prev_lower = false;
        } else if c.is_uppercase() && prev_lower {
            result.push('_');
            result.push(c.to_ascii_lowercase());
            prev_lower = false;
        } else {
            result.push(c.to_ascii_lowercase());
            prev_lower = c.is_lowercase();
        }
    }

    // Ensure it starts with a letter
    if result
        .chars()
        .next()
        .map(|c| c.is_numeric())
        .unwrap_or(true)
    {
        result = format!("icon_{}", result);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_to_identifier() {
        assert_eq!(
            filename_to_identifier("arrowDown-filled"),
            "arrow_down_filled"
        );
        assert_eq!(
            filename_to_identifier("Appliance-stroke"),
            "appliance_stroke"
        );
        assert_eq!(filename_to_identifier("Bank-filled"), "bank_filled");
        assert_eq!(filename_to_identifier("123icon"), "icon_123icon");
    }
}
