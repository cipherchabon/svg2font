# svg2font

Convert SVG icons to TTF icon font.

## Installation

```bash
cargo install svg2font
```

Or build from source:

```bash
cargo build --release
```

## Usage

```bash
svg2font generate [OPTIONS]

Options:
  -i, --input <INPUT>    Input directory containing SVG files [default: ./icons]
  -o, --output <OUTPUT>  Output directory for generated files [default: ./output]
  -n, --name <NAME>      Font family name [default: Icons]
  -p, --preview          Generate HTML preview page
  -v, --verbose          Enable verbose output
```

### Example

```bash
svg2font generate -i ./my-icons -o ./dist -n "MyAppIcons" --preview
# Output:
#   ./dist/myappicons.ttf
#   ./dist/myappicons_preview.html
```

## Preview

Use `--preview` to generate an interactive HTML page with:

- Visual grid of all icons
- Search/filter functionality
- Adjustable icon size
- Click to copy codepoint

The HTML file is self-contained (font embedded as base64) and can be opened directly in any browser.

## How it works

1. Parses all SVG files in the input directory using [usvg](https://github.com/linebender/resvg/tree/main/crates/usvg)
2. Converts SVG paths to font glyphs (cubic beziers are approximated to quadratic)
3. Assigns Unicode codepoints starting from U+E000 (Private Use Area)
4. Generates a valid TTF font using [write-fonts](https://github.com/googlefonts/fontations)

## SVG Requirements

- SVGs should be single-color icons
- Recommended size: 24x24 or similar square dimensions
- Paths will be scaled to fit the font's units-per-em (1000)

## License

MIT
