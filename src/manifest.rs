use crate::svg_parser::Icon;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

/// Generate a JSON manifest with icon metadata
pub fn generate_manifest(
    icons: &[Icon],
    font_name: &str,
    output_path: &Path,
) -> Result<()> {
    let json = generate_json(icons, font_name);

    let mut file = std::fs::File::create(output_path)
        .with_context(|| format!("Failed to create {}", output_path.display()))?;

    file.write_all(json.as_bytes())
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    Ok(())
}

fn generate_json(icons: &[Icon], font_name: &str) -> String {
    let mut icons_json = String::new();

    for (i, icon) in icons.iter().enumerate() {
        if i > 0 {
            icons_json.push_str(",\n");
        }
        icons_json.push_str(&format!(
            r#"    {{ "name": "{}", "filename": "{}", "codepoint": "{:04X}" }}"#,
            icon.name, icon.filename, icon.codepoint
        ));
    }

    format!(
        r#"{{
  "fontFamily": "{}",
  "icons": [
{}
  ]
}}"#,
        font_name, icons_json
    )
}
