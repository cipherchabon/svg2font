use anyhow::{Context, Result};
use kurbo::{BezPath, PathEl, Point, Shape};
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

/// Convert a usvg path to a kurbo BezPath, handling fill rules
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

    // Check if this path uses evenodd fill rule
    let fill_rule = path
        .fill()
        .map(|f| f.rule())
        .unwrap_or(usvg::FillRule::NonZero);

    if fill_rule == usvg::FillRule::EvenOdd {
        // For evenodd fill rule, we need to fix winding directions
        // TrueType uses non-zero winding, so inner contours must wind opposite to outer
        fix_evenodd_winding(&mut bez);
    }

    bez
}

/// Split a BezPath into individual contours (subpaths)
fn split_into_contours(path: &BezPath) -> Vec<BezPath> {
    let mut contours = Vec::new();
    let mut current = BezPath::new();

    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                if !current.elements().is_empty() {
                    contours.push(current);
                    current = BezPath::new();
                }
                current.move_to(*p);
            }
            PathEl::LineTo(p) => current.line_to(*p),
            PathEl::QuadTo(p1, p2) => current.quad_to(*p1, *p2),
            PathEl::CurveTo(p1, p2, p3) => current.curve_to(*p1, *p2, *p3),
            PathEl::ClosePath => current.close_path(),
        }
    }

    if !current.elements().is_empty() {
        contours.push(current);
    }

    contours
}

/// Calculate the signed area of a contour
/// Positive = counter-clockwise, Negative = clockwise
fn signed_area(contour: &BezPath) -> f64 {
    let mut area = 0.0;
    let mut first_point: Option<Point> = None;
    let mut prev_point: Option<Point> = None;

    for el in contour.elements() {
        match el {
            PathEl::MoveTo(p) => {
                first_point = Some(*p);
                prev_point = Some(*p);
            }
            PathEl::LineTo(p) => {
                if let Some(prev) = prev_point {
                    // Shoelace formula
                    area += (prev.x * p.y) - (p.x * prev.y);
                }
                prev_point = Some(*p);
            }
            PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
                // Approximate - just use end points for area calculation
                if let Some(prev) = prev_point {
                    area += (prev.x * p.y) - (p.x * prev.y);
                }
                prev_point = Some(*p);
            }
            PathEl::ClosePath => {
                if let (Some(prev), Some(first)) = (prev_point, first_point) {
                    area += (prev.x * first.y) - (first.x * prev.y);
                }
            }
        }
    }

    area / 2.0
}

/// Reverse the winding direction of a contour
fn reverse_contour(contour: &BezPath) -> BezPath {
    let mut points: Vec<(PathEl, Point)> = Vec::new();
    let mut current_point = Point::ZERO;
    let mut first_point = Point::ZERO;
    let mut has_close = false;

    // Collect all points and their element types
    for el in contour.elements() {
        match el {
            PathEl::MoveTo(p) => {
                first_point = *p;
                current_point = *p;
            }
            PathEl::LineTo(p) => {
                points.push((PathEl::LineTo(current_point), *p));
                current_point = *p;
            }
            PathEl::QuadTo(p1, p2) => {
                points.push((PathEl::QuadTo(*p1, current_point), *p2));
                current_point = *p2;
            }
            PathEl::CurveTo(p1, p2, p3) => {
                points.push((PathEl::CurveTo(*p2, *p1, current_point), *p3));
                current_point = *p3;
            }
            PathEl::ClosePath => {
                has_close = true;
                if current_point != first_point {
                    points.push((PathEl::LineTo(current_point), first_point));
                }
            }
        }
    }

    // Build reversed path
    let mut reversed = BezPath::new();

    if points.is_empty() {
        reversed.move_to(first_point);
        return reversed;
    }

    // Start from the last point
    let last_point = points.last().map(|(_, p)| *p).unwrap_or(first_point);
    reversed.move_to(last_point);

    // Add elements in reverse order
    for (el, _) in points.iter().rev() {
        match el {
            PathEl::LineTo(p) => reversed.line_to(*p),
            PathEl::QuadTo(p1, p2) => reversed.quad_to(*p1, *p2),
            PathEl::CurveTo(p1, p2, p3) => reversed.curve_to(*p1, *p2, *p3),
            _ => {}
        }
    }

    if has_close {
        reversed.close_path();
    }

    reversed
}

/// Check if a point is inside a contour using ray casting
fn point_in_contour(point: Point, contour: &BezPath) -> bool {
    let mut inside = false;
    let mut prev_point: Option<Point> = None;
    let mut first_point: Option<Point> = None;

    for el in contour.elements() {
        match el {
            PathEl::MoveTo(p) => {
                first_point = Some(*p);
                prev_point = Some(*p);
            }
            PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
                if let Some(prev) = prev_point {
                    // Ray casting algorithm
                    if (prev.y > point.y) != (p.y > point.y) {
                        let x_intersect =
                            prev.x + (point.y - prev.y) / (p.y - prev.y) * (p.x - prev.x);
                        if point.x < x_intersect {
                            inside = !inside;
                        }
                    }
                }
                prev_point = Some(*p);
            }
            PathEl::ClosePath => {
                if let (Some(prev), Some(first)) = (prev_point, first_point) {
                    if (prev.y > point.y) != (first.y > point.y) {
                        let x_intersect =
                            prev.x + (point.y - prev.y) / (first.y - prev.y) * (first.x - prev.x);
                        if point.x < x_intersect {
                            inside = !inside;
                        }
                    }
                }
            }
        }
    }

    inside
}

/// Fix winding directions for evenodd fill rule to work with non-zero winding
fn fix_evenodd_winding(path: &mut BezPath) {
    let contours = split_into_contours(path);

    if contours.len() <= 1 {
        return; // Nothing to fix for single contours
    }

    // Calculate signed areas and bounding boxes for all contours
    let mut contour_info: Vec<(BezPath, f64, kurbo::Rect)> = contours
        .into_iter()
        .map(|c| {
            let area = signed_area(&c);
            let bbox = c.bounding_box();
            (c, area, bbox)
        })
        .collect();

    // Sort by bounding box area (descending) - larger contours are likely outer
    contour_info.sort_by(|a, b| {
        let area_a = a.2.width() * a.2.height();
        let area_b = b.2.width() * b.2.height();
        area_b.partial_cmp(&area_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Determine nesting level for each contour
    let mut fixed_contours: Vec<BezPath> = Vec::new();

    for i in 0..contour_info.len() {
        let (contour, area, bbox) = &contour_info[i];

        // Count how many contours this one is inside of
        let mut nesting_level = 0;
        let center = Point::new(bbox.x0 + bbox.width() / 2.0, bbox.y0 + bbox.height() / 2.0);

        for (other_contour, _, other_bbox) in contour_info.iter().take(i) {
            // Quick check: if bounding box doesn't contain our center, skip
            if other_bbox.contains(center) && point_in_contour(center, other_contour) {
                nesting_level += 1;
            }
        }

        // For TrueType non-zero winding:
        // - Outer contours (even nesting level) should be clockwise (negative area)
        // - Inner contours (odd nesting level) should be counter-clockwise (positive area)
        let should_be_clockwise = nesting_level % 2 == 0;
        let is_clockwise = *area < 0.0;

        let fixed_contour = if should_be_clockwise != is_clockwise {
            reverse_contour(contour)
        } else {
            contour.clone()
        };

        fixed_contours.push(fixed_contour);
    }

    // Rebuild the path
    *path = BezPath::new();
    for contour in fixed_contours {
        for el in contour.elements() {
            path.push(*el);
        }
    }
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
