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

        :root {{
            --bg-primary: #ffffff;
            --bg-secondary: #f5f5f5;
            --bg-card: #ffffff;
            --border-color: #e0e0e0;
            --border-hover: #ccc;
            --text-primary: #1a1a1a;
            --text-secondary: #666;
            --text-muted: #999;
            --icon-color: #1a1a1a;
            --input-bg: #fff;
            --toast-bg: #333;
            --toast-color: #fff;
        }}

        @media (prefers-color-scheme: dark) {{
            :root:not([data-theme="light"]) {{
                --bg-primary: #0a0a0a;
                --bg-secondary: #111;
                --bg-card: #151515;
                --border-color: #222;
                --border-hover: #333;
                --text-primary: #e0e0e0;
                --text-secondary: #888;
                --text-muted: #555;
                --icon-color: #fff;
                --input-bg: #1a1a1a;
                --toast-bg: #444;
                --toast-color: #fff;
            }}
        }}

        [data-theme="dark"] {{
            --bg-primary: #0a0a0a;
            --bg-secondary: #111;
            --bg-card: #151515;
            --border-color: #222;
            --border-hover: #333;
            --text-primary: #e0e0e0;
            --text-secondary: #888;
            --text-muted: #555;
            --icon-color: #fff;
            --input-bg: #1a1a1a;
            --toast-bg: #444;
            --toast-color: #fff;
        }}

        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            min-height: 100vh;
            transition: background 0.2s, color 0.2s;
        }}

        .header {{
            background: var(--bg-secondary);
            border-bottom: 1px solid var(--border-color);
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
            color: var(--text-secondary);
            font-size: 0.875rem;
        }}

        .controls {{
            display: flex;
            gap: 1rem;
            align-items: center;
            flex-wrap: wrap;
        }}

        .search-box {{
            background: var(--input-bg);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 0.5rem 1rem;
            color: var(--text-primary);
            font-size: 0.875rem;
            width: 200px;
        }}

        .search-box:focus {{
            outline: none;
            border-color: var(--border-hover);
        }}

        .size-control {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}

        .size-control label {{
            font-size: 0.875rem;
            color: var(--text-secondary);
        }}

        .size-slider {{
            width: 100px;
        }}

        .theme-toggle {{
            background: var(--input-bg);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 0.5rem;
            cursor: pointer;
            font-size: 1.25rem;
            line-height: 1;
            transition: border-color 0.2s;
        }}

        .theme-toggle:hover {{
            border-color: var(--border-hover);
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
            background: var(--bg-card);
            border: 1px solid var(--border-color);
            border-radius: 12px;
            padding: 1.25rem;
            text-align: center;
            cursor: pointer;
            transition: all 0.2s ease;
        }}

        .icon-card:hover {{
            border-color: var(--border-hover);
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
            color: var(--icon-color);
        }}

        .icon-name {{
            font-size: 0.75rem;
            color: var(--text-secondary);
            word-break: break-word;
            margin-bottom: 0.25rem;
        }}

        .icon-code {{
            font-size: 0.625rem;
            color: var(--text-muted);
            font-family: monospace;
        }}

        .toast {{
            position: fixed;
            bottom: 2rem;
            left: 50%;
            transform: translateX(-50%) translateY(100px);
            background: var(--toast-bg);
            color: var(--toast-color);
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
            color: var(--text-muted);
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
                <button class="theme-toggle" id="theme-toggle" title="Toggle theme">
                    <span class="theme-icon">🌙</span>
                </button>
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
        const themeToggle = document.getElementById('theme-toggle');
        const themeIcon = themeToggle.querySelector('.theme-icon');

        // Theme handling
        function getSystemTheme() {{
            return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        }}

        function getCurrentTheme() {{
            return document.documentElement.dataset.theme || getSystemTheme();
        }}

        function setTheme(theme) {{
            document.documentElement.dataset.theme = theme;
            themeIcon.textContent = theme === 'dark' ? '☀️' : '🌙';
            localStorage.setItem('theme', theme);
        }}

        // Initialize theme
        const savedTheme = localStorage.getItem('theme');
        if (savedTheme) {{
            setTheme(savedTheme);
        }} else {{
            themeIcon.textContent = getSystemTheme() === 'dark' ? '☀️' : '🌙';
        }}

        themeToggle.addEventListener('click', () => {{
            const newTheme = getCurrentTheme() === 'dark' ? 'light' : 'dark';
            setTheme(newTheme);
        }});

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
