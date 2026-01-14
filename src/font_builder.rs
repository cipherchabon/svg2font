use crate::svg_parser::Icon;
use anyhow::{Context, Result};
use kurbo::{Affine, BezPath, CubicBez, ParamCurve, PathEl, Point, QuadBez};
use std::path::Path;
use write_fonts::{
    tables::{
        cmap::Cmap,
        glyf::{GlyfLocaBuilder, SimpleGlyph},
        head::{Head, MacStyle},
        hhea::Hhea,
        hmtx::Hmtx,
        maxp::Maxp,
        name::{Name, NameRecord},
        os2::Os2,
        post::Post,
        vmtx::LongMetric,
    },
    types::{FWord, Fixed, GlyphId, NameId, UfWord},
    FontBuilder,
};

/// Units per em for the generated font
const UNITS_PER_EM: u16 = 1000;

/// Build a TTF font from a list of icons
pub fn build_font(icons: &[Icon], font_name: &str, output_path: &Path, verbose: bool) -> Result<()> {
    // Build glyf and loca tables
    let mut glyf_builder = GlyfLocaBuilder::new();

    // Add .notdef glyph (required, empty)
    glyf_builder.add_glyph(&empty_glyph())?;

    // Track metrics for hmtx
    let mut metrics: Vec<LongMetric> = vec![LongMetric {
        advance: UNITS_PER_EM,
        side_bearing: 0,
    }];

    for icon in icons {
        if verbose {
            println!("  Building glyph: {} (U+{:04X})", icon.name, icon.codepoint);
        }

        // Convert SVG path to font glyph
        let glyph = svg_path_to_glyph(&icon.path, icon.width, icon.height)?;
        glyf_builder.add_glyph(&glyph)?;

        metrics.push(LongMetric {
            advance: UNITS_PER_EM,
            side_bearing: 0,
        });
    }

    let (glyf, loca, loca_format) = glyf_builder.build();

    // Build cmap table (character to glyph mapping)
    let cmap = build_cmap(icons)?;

    // Build head table
    let mut head = build_head();
    head.index_to_loc_format = loca_format as i16;

    // Build hhea table
    let hhea = build_hhea(icons.len() as u16 + 1);

    // Build hmtx table
    let hmtx = Hmtx::new(metrics, vec![]);

    // Build maxp table
    let maxp = Maxp {
        num_glyphs: icons.len() as u16 + 1, // +1 for .notdef
        ..Default::default()
    };

    // Build name table
    let name = build_name(font_name);

    // Build OS/2 table
    let os2 = build_os2(icons);

    // Build post table
    let post = build_post();

    // Assemble the font
    let font_data = FontBuilder::new()
        .add_table(&head)?
        .add_table(&hhea)?
        .add_table(&maxp)?
        .add_table(&os2)?
        .add_table(&hmtx)?
        .add_table(&cmap)?
        .add_table(&name)?
        .add_table(&post)?
        .add_table(&loca)?
        .add_table(&glyf)?
        .build();

    std::fs::write(output_path, font_data)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    Ok(())
}

/// Create an empty .notdef glyph
fn empty_glyph() -> SimpleGlyph {
    SimpleGlyph::default()
}

/// Convert an SVG BezPath to a font SimpleGlyph
fn svg_path_to_glyph(path: &BezPath, svg_width: f64, svg_height: f64) -> Result<SimpleGlyph> {
    // Calculate scale to fit in UNITS_PER_EM
    let scale = UNITS_PER_EM as f64 / svg_width.max(svg_height);

    // Transform: scale and flip Y axis (SVG is Y-down, fonts are Y-up)
    // Also center vertically
    let transform = Affine::new([
        scale,
        0.0,
        0.0,
        -scale,            // Flip Y
        0.0,
        svg_height * scale, // Move origin
    ]);

    let transformed = transform * path.clone();

    // Convert cubic beziers to quadratic (TTF only supports quadratic)
    let quadratic_path = cubic_to_quadratic(&transformed);

    // Create glyph from path
    if quadratic_path.elements().is_empty() {
        return Ok(SimpleGlyph::default());
    }

    SimpleGlyph::from_bezpath(&quadratic_path)
        .map_err(|e| anyhow::anyhow!("Failed to create glyph: {:?}", e))
}

/// Convert cubic bezier curves to quadratic approximations
/// TTF glyphs only support quadratic beziers
fn cubic_to_quadratic(path: &BezPath) -> BezPath {
    let mut result = BezPath::new();
    let mut current_point = Point::ZERO;

    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                result.move_to(*p);
                current_point = *p;
            }
            PathEl::LineTo(p) => {
                result.line_to(*p);
                current_point = *p;
            }
            PathEl::QuadTo(p1, p2) => {
                result.quad_to(*p1, *p2);
                current_point = *p2;
            }
            PathEl::CurveTo(p1, p2, p3) => {
                // Approximate cubic with multiple quadratics
                let cubic = CubicBez::new(current_point, *p1, *p2, *p3);
                approximate_cubic_with_quadratics(&cubic, &mut result);
                current_point = *p3;
            }
            PathEl::ClosePath => {
                result.close_path();
            }
        }
    }

    result
}

/// Approximate a cubic bezier with quadratic beziers
/// Uses subdivision for better accuracy
fn approximate_cubic_with_quadratics(cubic: &CubicBez, path: &mut BezPath) {
    // Simple approximation: use the midpoint method
    // For more accuracy, we could use adaptive subdivision

    let tolerance = 1.0; // Error tolerance in font units

    // Try to fit with a single quadratic first
    let midpoint = cubic.eval(0.5);
    let quad_control = Point::new(
        (cubic.p1.x + cubic.p2.x) / 2.0,
        (cubic.p1.y + cubic.p2.y) / 2.0,
    );

    let quad = QuadBez::new(cubic.p0, quad_control, cubic.p3);
    let quad_mid = quad.eval(0.5);

    let error = (midpoint.x - quad_mid.x).abs() + (midpoint.y - quad_mid.y).abs();

    if error < tolerance {
        // Single quadratic is good enough
        path.quad_to(quad_control, cubic.p3);
    } else {
        // Subdivide the cubic and approximate each half
        let (left, right) = subdivide_cubic(cubic);
        approximate_cubic_with_quadratics(&left, path);
        approximate_cubic_with_quadratics(&right, path);
    }
}

/// Subdivide a cubic bezier at t=0.5
fn subdivide_cubic(cubic: &CubicBez) -> (CubicBez, CubicBez) {
    let p01 = midpoint(cubic.p0, cubic.p1);
    let p12 = midpoint(cubic.p1, cubic.p2);
    let p23 = midpoint(cubic.p2, cubic.p3);
    let p012 = midpoint(p01, p12);
    let p123 = midpoint(p12, p23);
    let p0123 = midpoint(p012, p123);

    let left = CubicBez::new(cubic.p0, p01, p012, p0123);
    let right = CubicBez::new(p0123, p123, p23, cubic.p3);

    (left, right)
}

fn midpoint(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

/// Build the cmap table (character to glyph mapping)
fn build_cmap(icons: &[Icon]) -> Result<Cmap> {
    // Build mappings from codepoint to glyph ID
    let mappings: Vec<(char, GlyphId)> = icons
        .iter()
        .enumerate()
        .filter_map(|(i, icon)| {
            // Convert codepoint to char, skip if invalid
            char::from_u32(icon.codepoint).map(|c| {
                let glyph_id = GlyphId::new((i + 1) as u32); // +1 because .notdef is 0
                (c, glyph_id)
            })
        })
        .collect();

    // Create cmap from mappings
    Cmap::from_mappings(mappings)
        .map_err(|e| anyhow::anyhow!("Failed to create cmap: {:?}", e))
}

/// Build the head table
fn build_head() -> Head {
    Head {
        font_revision: Fixed::from_f64(1.0),
        units_per_em: UNITS_PER_EM,
        created: Default::default(),
        modified: Default::default(),
        mac_style: MacStyle::empty(),
        lowest_rec_ppem: 8,
        index_to_loc_format: 1, // Long offsets (will be updated)
        ..Default::default()
    }
}

/// Build the hhea table
fn build_hhea(num_glyphs: u16) -> Hhea {
    Hhea {
        ascender: FWord::new(800),
        descender: FWord::new(-200),
        line_gap: FWord::new(0),
        advance_width_max: UfWord::new(UNITS_PER_EM),
        min_left_side_bearing: FWord::new(0),
        min_right_side_bearing: FWord::new(0),
        x_max_extent: FWord::new(UNITS_PER_EM as i16),
        caret_slope_rise: 1,
        caret_slope_run: 0,
        caret_offset: 0,
        number_of_h_metrics: num_glyphs,
        ..Default::default()
    }
}

/// Build the name table
fn build_name(font_name: &str) -> Name {
    let mut name = Name::default();

    // Add name records for all required name IDs
    name.name_record.push(create_name_record(
        NameId::COPYRIGHT_NOTICE,
        "Generated by svg2font",
    ));
    name.name_record.push(create_name_record(
        NameId::FAMILY_NAME,
        font_name,
    ));
    name.name_record.push(create_name_record(
        NameId::SUBFAMILY_NAME,
        "Regular",
    ));
    name.name_record.push(create_name_record(
        NameId::UNIQUE_ID,
        &format!("svg2font: {}", font_name),
    ));
    name.name_record.push(create_name_record(
        NameId::FULL_NAME,
        font_name,
    ));
    name.name_record.push(create_name_record(
        NameId::VERSION_STRING,
        "Version 1.0",
    ));
    name.name_record.push(create_name_record(
        NameId::POSTSCRIPT_NAME,
        &font_name.replace(' ', ""),
    ));

    name
}

fn create_name_record(name_id: NameId, value: &str) -> NameRecord {
    NameRecord {
        platform_id: 3,  // Windows
        encoding_id: 1,  // Unicode BMP
        language_id: 0x409, // English US
        name_id,
        string: value.to_string().into(),
    }
}

/// Build the OS/2 table
fn build_os2(_icons: &[Icon]) -> Os2 {
    Os2 {
        x_avg_char_width: UNITS_PER_EM as i16,
        us_weight_class: 400, // Normal
        us_width_class: 5,    // Medium
        fs_type: 0,           // Installable
        y_subscript_x_size: 650,
        y_subscript_y_size: 600,
        y_subscript_x_offset: 0,
        y_subscript_y_offset: 75,
        y_superscript_x_size: 650,
        y_superscript_y_size: 600,
        y_superscript_x_offset: 0,
        y_superscript_y_offset: 350,
        y_strikeout_size: 50,
        y_strikeout_position: 300,
        s_typo_ascender: 800,
        s_typo_descender: -200,
        s_typo_line_gap: 0,
        us_win_ascent: 1000,
        us_win_descent: 200,
        ul_unicode_range_1: 0,
        ul_unicode_range_2: 0,
        ul_unicode_range_3: 0,
        ul_unicode_range_4: 1 << 28, // Private Use Area
        ul_code_page_range_1: Some(1), // Latin 1
        ul_code_page_range_2: Some(0),
        sx_height: Some(500),
        s_cap_height: Some(700),
        us_default_char: Some(0),
        us_break_char: Some(32),
        us_max_context: Some(0),
        us_lower_optical_point_size: None,
        us_upper_optical_point_size: None,
        ..Default::default()
    }
}

/// Build the post table
fn build_post() -> Post {
    Post::new_v2(std::iter::empty::<&str>())
}
