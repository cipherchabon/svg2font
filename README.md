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
  -v, --verbose          Enable verbose output
```

### Example

```bash
svg2font generate -i ./my-icons -o ./dist -n "MyAppIcons"
# Output: ./dist/myappicons.ttf
```

## How it works

1. Parses all SVG files in the input directory using [usvg](https://github.com/AltRandom/resvg/tree/master/crates/usvg)
2. Converts SVG paths to font glyphs (cubic beziers are approximated to quadratic)
3. Assigns Unicode codepoints starting from U+E000 (Private Use Area)
4. Generates a valid TTF font using [write-fonts](https://github.com/googlefonts/fontations)

## SVG Requirements

- SVGs should be single-color icons
- Recommended size: 24x24 or similar square dimensions
- Paths will be scaled to fit the font's units-per-em (1000)

## License

MIT
