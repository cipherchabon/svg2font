use crate::svg_parser::Icon;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

/// Generate an HTML preview page for the icon font
pub fn generate_preview(
    icons: &[Icon],
    font_name: &str,
    ttf_path: &Path,
    output_path: &Path,
) -> Result<()> {
    // Read TTF and encode as base64
    let ttf_data = std::fs::read(ttf_path)
        .with_context(|| format!("Failed to read {}", ttf_path.display()))?;
    let ttf_base64 = base64_encode(&ttf_data);

    let html = generate_html(icons, font_name, &ttf_base64);

    let mut file = std::fs::File::create(output_path)
        .with_context(|| format!("Failed to create {}", output_path.display()))?;

    file.write_all(html.as_bytes())
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    Ok(())
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

fn generate_html(icons: &[Icon], font_name: &str, ttf_base64: &str) -> String {
    let mut icons_html = String::new();

    for icon in icons {
        icons_html.push_str(&format!(
            r#"
        <div class="icon-card" data-name="{name}" data-codepoint="{codepoint:04X}">
            <div class="icon-glyph">&#x{codepoint:04X};</div>
            <div class="icon-name">{name}</div>
            <div class="icon-code">U+{codepoint:04X}</div>
        </div>"#,
            name = icon.filename,
            codepoint = icon.codepoint
        ));
    }

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{font_name} - Icon Font Preview</title>
    <style>
        @font-face {{
            font-family: '{font_name}';
            src: url('data:font/truetype;base64,{ttf_base64}') format('truetype');
            font-weight: normal;
            font-style: normal;
        }}

        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0a0a0a;
            color: #e0e0e0;
            min-height: 100vh;
        }}

        .header {{
            background: #111;
            border-bottom: 1px solid #222;
            padding: 1.5rem 2rem;
            position: sticky;
            top: 0;
            z-index: 100;
        }}

        .header-content {{
            max-width: 1400px;
            margin: 0 auto;
            display: flex;
            justify-content: space-between;
            align-items: center;
            gap: 1rem;
            flex-wrap: wrap;
        }}

        h1 {{
            font-size: 1.5rem;
            font-weight: 600;
        }}

        .stats {{
            color: #888;
            font-size: 0.875rem;
        }}

        .controls {{
            display: flex;
            gap: 1rem;
            align-items: center;
            flex-wrap: wrap;
        }}

        .search-box {{
            background: #1a1a1a;
            border: 1px solid #333;
            border-radius: 8px;
            padding: 0.5rem 1rem;
            color: #e0e0e0;
            font-size: 0.875rem;
            width: 200px;
        }}

        .search-box:focus {{
            outline: none;
            border-color: #555;
        }}

        .size-control {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}

        .size-control label {{
            font-size: 0.875rem;
            color: #888;
        }}

        .size-slider {{
            width: 100px;
            accent-color: #666;
        }}

        .container {{
            max-width: 1400px;
            margin: 0 auto;
            padding: 2rem;
        }}

        .grid {{
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
            gap: 1rem;
        }}

        .icon-card {{
            background: #151515;
            border: 1px solid #222;
            border-radius: 12px;
            padding: 1.25rem;
            text-align: center;
            cursor: pointer;
            transition: all 0.2s ease;
        }}

        .icon-card:hover {{
            background: #1a1a1a;
            border-color: #333;
            transform: translateY(-2px);
        }}

        .icon-card.hidden {{
            display: none;
        }}

        .icon-glyph {{
            font-family: '{font_name}';
            font-size: var(--icon-size, 32px);
            line-height: 1;
            margin-bottom: 0.75rem;
            color: #fff;
        }}

        .icon-name {{
            font-size: 0.75rem;
            color: #888;
            word-break: break-word;
            margin-bottom: 0.25rem;
        }}

        .icon-code {{
            font-size: 0.625rem;
            color: #555;
            font-family: monospace;
        }}

        .toast {{
            position: fixed;
            bottom: 2rem;
            left: 50%;
            transform: translateX(-50%) translateY(100px);
            background: #333;
            color: #fff;
            padding: 0.75rem 1.5rem;
            border-radius: 8px;
            font-size: 0.875rem;
            opacity: 0;
            transition: all 0.3s ease;
            z-index: 1000;
        }}

        .toast.show {{
            transform: translateX(-50%) translateY(0);
            opacity: 1;
        }}

        .no-results {{
            grid-column: 1 / -1;
            text-align: center;
            padding: 3rem;
            color: #666;
        }}
    </style>
</head>
<body>
    <header class="header">
        <div class="header-content">
            <div>
                <h1>{font_name}</h1>
                <p class="stats">{icon_count} icons</p>
            </div>
            <div class="controls">
                <input type="text" class="search-box" placeholder="Search icons..." id="search">
                <div class="size-control">
                    <label>Size:</label>
                    <input type="range" class="size-slider" id="size" min="16" max="64" value="32">
                    <span id="size-value">32px</span>
                </div>
            </div>
        </div>
    </header>

    <main class="container">
        <div class="grid" id="grid">
            {icons_html}
        </div>
    </main>

    <div class="toast" id="toast">Copied!</div>

    <script>
        const grid = document.getElementById('grid');
        const search = document.getElementById('search');
        const sizeSlider = document.getElementById('size');
        const sizeValue = document.getElementById('size-value');
        const toast = document.getElementById('toast');

        // Search functionality
        search.addEventListener('input', (e) => {{
            const query = e.target.value.toLowerCase();
            document.querySelectorAll('.icon-card').forEach(card => {{
                const name = card.dataset.name.toLowerCase();
                const code = card.dataset.codepoint.toLowerCase();
                const matches = name.includes(query) || code.includes(query);
                card.classList.toggle('hidden', !matches);
            }});

            const visible = document.querySelectorAll('.icon-card:not(.hidden)').length;
            const noResults = document.querySelector('.no-results');
            if (visible === 0 && !noResults) {{
                grid.insertAdjacentHTML('beforeend', '<div class="no-results">No icons found</div>');
            }} else if (visible > 0 && noResults) {{
                noResults.remove();
            }}
        }});

        // Size control
        sizeSlider.addEventListener('input', (e) => {{
            const size = e.target.value;
            document.documentElement.style.setProperty('--icon-size', size + 'px');
            sizeValue.textContent = size + 'px';
        }});

        // Copy on click
        grid.addEventListener('click', (e) => {{
            const card = e.target.closest('.icon-card');
            if (!card) return;

            const codepoint = card.dataset.codepoint;
            const text = `U+${{codepoint}}`;

            navigator.clipboard.writeText(text).then(() => {{
                toast.textContent = `Copied ${{text}}`;
                toast.classList.add('show');
                setTimeout(() => toast.classList.remove('show'), 2000);
            }});
        }});
    </script>
</body>
</html>"##,
        font_name = font_name,
        ttf_base64 = ttf_base64,
        icon_count = icons.len(),
        icons_html = icons_html
    )
}
