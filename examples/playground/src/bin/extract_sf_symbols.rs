//! Developer tool (macOS only): extracts SF Symbols from the operating system
//! as vector SVGs for `liquid-glass-ui`'s icon assets.
//!
//! `liquid_glass_native::system_symbol_pdf` renders a symbol through
//! `CoreUI`'s `CUINamedVectorGlyph` into a vector PDF; this binary walks the
//! PDF content stream and rewrites the path operators as an SVG `<path>`.
//!
//! Usage: `cargo run -p liquid-glass-playground --bin extract-sf-symbols`

#[cfg(target_os = "macos")]
use std::path::PathBuf;

/// (SF Symbol name, asset file stem) pairs used by the UI components.
#[cfg(target_os = "macos")]
const SYMBOLS: &[(&str, &str)] = &[
    ("gear", "gear"),
    ("bell.fill", "bell"),
    ("hand.raised.fill", "privacy"),
    ("magnifyingglass", "search"),
    ("chevron.left", "chevron_left"),
    ("chevron.right", "chevron_right"),
    ("questionmark.circle", "question"),
    ("info.circle", "info"),
    ("laptopcomputer", "laptop"),
    ("arrow.triangle.2.circlepath", "software_update"),
    ("keyboard", "keyboard"),
    ("computermouse", "mouse"),
    ("speaker.wave.2", "sound"),
    ("wifi", "wifi"),
];

/// Bespoke (non-SF-Symbol) assets extracted from feature bundles' compiled
/// asset catalogs: (catalog path, asset name, output stem, rendition scale).
#[cfg(target_os = "macos")]
const BUNDLE_ASSETS: &[(&str, &str, &str, f64)] = &[
    (
        "/System/Library/ExtensionKit/Extensions/Bluetooth.appex/Contents/Resources/Assets.car",
        "BluetoothIcon",
        "bluetooth",
        1.0,
    ),
    (
        "/System/Library/ExtensionKit/Extensions/SoftwareUpdateSettingsExtension.appex/Contents/Resources/Assets.car",
        "softwareupdate",
        "software_update_badge",
        2.0,
    ),
    (
        "/System/Library/ExtensionKit/Extensions/SecurityPrivacyExtension.appex/Contents/Resources/Assets.car",
        "FDEIcon",
        "filevault",
        2.0,
    ),
];

/// Private vector glyphs that ship inside `CoreGlyphsPrivate.bundle` instead
/// of the public SF Symbols catalog: (catalog path, glyph name, output stem).
/// The real Appearance settings extension declares `ISSymbolName: appearance`
/// in its Info.plist; `name_aliases.strings` maps that to `appearance.darkmode`.
/// Private vector glyphs that ship inside `CoreGlyphsPrivate.bundle` instead
/// of the public SF Symbols catalog: (catalog path, glyph name, output stem).
/// The real Appearance settings extension declares `ISSymbolName: appearance`
/// in its Info.plist; the system swaps between the light/dark renditions with
/// the active color scheme, so both are extracted.
#[cfg(target_os = "macos")]
const PRIVATE_GLYPHS: &[(&str, &str, &str)] = &[
    (
        "/System/Library/CoreServices/CoreGlyphsPrivate.bundle/Contents/Resources/Assets.car",
        "appearance.lightmode",
        "appearance",
    ),
    (
        "/System/Library/CoreServices/CoreGlyphsPrivate.bundle/Contents/Resources/Assets.car",
        "appearance.darkmode",
        "appearance_dark",
    ),
];

/// Graphic icons resolved through `IconServices` by type identifier (used by
/// the real Notifications pane): (type identifier, output stem). The system
/// returns a 1024px rendition; `sips` downsamples it to a working size.
#[cfg(target_os = "macos")]
const GRAPHIC_ICONS: &[(&str, &str)] = &[("com.apple.graphic-icon.notifications", "notifications")];

fn main() {
    #[cfg(not(target_os = "macos"))]
    eprintln!("extract-sf-symbols is a macOS-only developer tool");

    #[cfg(target_os = "macos")]
    {
        let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/liquid-glass-ui/assets/icons");
        std::fs::create_dir_all(&out_dir).expect("create assets/icons");

        let mut failures = 0;
        for (symbol, stem) in SYMBOLS {
            match extract(symbol) {
                Ok(svg) => {
                    let path = out_dir.join(format!("{stem}.svg"));
                    std::fs::write(&path, &svg).expect("write svg");
                    println!("ok   {symbol:24} -> {} ({} bytes)", path.display(), svg.len());
                }
                Err(error) => {
                    failures += 1;
                    eprintln!("FAIL {symbol:24} {error}");
                }
            }
        }
        for (car, name, stem) in PRIVATE_GLYPHS {
            match liquid_glass_native::glyph_pdf(car, name)
                .ok_or_else(|| "glyph_pdf returned None".to_owned())
                .and_then(|pdf| svg::emit(&pdf::Page::parse(&pdf)?))
            {
                Ok(svg) => {
                    let path = out_dir.join(format!("{stem}.svg"));
                    std::fs::write(&path, &svg).expect("write svg");
                    println!("ok   {name:24} -> {} ({} bytes)", path.display(), svg.len());
                }
                Err(error) => {
                    failures += 1;
                    eprintln!("FAIL {name:24} {error}");
                }
            }
        }
        for (identifier, stem) in GRAPHIC_ICONS {
            if let Some(png) = liquid_glass_native::graphic_icon_png(identifier) {
                let path = out_dir.join(format!("{stem}.png"));
                std::fs::write(&path, &png).expect("write png");
                // 1024px is needlessly heavy for a 28pt sidebar tile.
                limit_png_size(&path, 128);
                println!("ok   {identifier:40} -> {}", path.display());
            } else {
                failures += 1;
                eprintln!("FAIL {identifier:40} graphic_icon_png returned None");
            }
        }
        for (car, name, stem, scale) in BUNDLE_ASSETS {
            if let Some(png) = liquid_glass_native::named_asset_png(car, name, *scale) {
                let path = out_dir.join(format!("{stem}.png"));
                std::fs::write(&path, &png).expect("write png");
                limit_png_size(&path, 128);
                println!("ok   {name:24} -> {} ({} bytes)", path.display(), png.len());
            } else {
                failures += 1;
                eprintln!("FAIL {name:24} named_asset_png returned None for {car}");
            }
        }
        if failures > 0 {
            std::process::exit(2);
        }
    }
}

#[cfg(target_os = "macos")]
fn extract(symbol: &str) -> Result<String, String> {
    let pdf =
        liquid_glass_native::system_symbol_pdf(symbol).ok_or("system_symbol_pdf returned None")?;
    let page = pdf::Page::parse(&pdf)?;
    svg::emit(&page)
}

/// Caps a PNG's larger dimension at `max_px` via `sips` (no upscaling).
#[cfg(target_os = "macos")]
fn limit_png_size(path: &std::path::Path, max_px: u32) {
    let Ok(output) = std::process::Command::new("sips")
        .args(["-g", "pixelWidth"])
        .arg(path)
        .output()
    else {
        return;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let width: u32 = stdout
        .split_whitespace()
        .next_back()
        .and_then(|token| token.parse().ok())
        .unwrap_or(0);
    if width > max_px {
        std::process::Command::new("sips")
            .args(["-Z", &max_px.to_string()])
            .arg(path)
            .status()
            .expect("run sips");
    }
}

/// Minimal PDF reader: enough to pull the page's content stream and media box
/// out of a CoreGraphics-generated single-page document. Operates on raw
/// bytes throughout so binary header bytes never shift offsets.
#[cfg(target_os = "macos")]
mod pdf {
    use std::io::Read as _;

    pub struct Page {
        pub height: f64,
        pub content: Vec<u8>,
    }

    impl Page {
        pub fn parse(pdf: &[u8]) -> Result<Self, String> {
            let (_width, height) = media_box(pdf)?;
            let contents_ref = contents_object_ref(pdf)?;
            let content = object_stream(pdf, contents_ref)?;
            Ok(Self { height, content })
        }
    }

    fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
        haystack.get(from..)?.windows(needle.len()).position(|w| w == needle).map(|i| i + from)
    }

    fn parse_numbers(slice: &[u8]) -> Vec<f64> {
        String::from_utf8_lossy(slice)
            .split_whitespace()
            .filter_map(|token| token.parse().ok())
            .collect()
    }

    /// First `/MediaBox [ _ _ w h ]` in the file (CoreGraphics writes one).
    fn media_box(pdf: &[u8]) -> Result<(f64, f64), String> {
        let start = find(pdf, b"/MediaBox", 0).ok_or("no /MediaBox")?;
        let open = find(pdf, b"[", start).ok_or("bad /MediaBox")?;
        let close = find(pdf, b"]", open).ok_or("bad /MediaBox")?;
        let numbers = parse_numbers(&pdf[open + 1..close]);
        if numbers.len() != 4 {
            return Err("bad /MediaBox numbers".into());
        }
        Ok((numbers[2] - numbers[0], numbers[3] - numbers[1]))
    }

    /// The object number referenced by the `/Type /Page` object's `/Contents`.
    fn contents_object_ref(pdf: &[u8]) -> Result<u32, String> {
        let page = find(pdf, b"/Type /Page", 0).ok_or("no /Page object")?;
        let contents = find(pdf, b"/Contents", page).ok_or("page has no /Contents")?;
        let mut digits = Vec::new();
        for &byte in &pdf[contents + b"/Contents".len()..] {
            if byte.is_ascii_digit() {
                digits.push(byte);
            } else if !digits.is_empty() {
                break;
            }
        }
        String::from_utf8(digits)
            .map_err(|_| "bad /Contents reference".to_owned())?
            .parse()
            .map_err(|_| "bad /Contents reference".to_owned())
    }

    /// Inflates (or copies) the stream bytes of object `number`.
    fn object_stream(pdf: &[u8], number: u32) -> Result<Vec<u8>, String> {
        let marker = format!("\n{number} 0 obj");
        let start = find(pdf, marker.as_bytes(), 0).ok_or("content object missing")?;
        let dict_end = find(pdf, b">>", start).ok_or("bad object dict")?;
        let dictionary = &pdf[start..dict_end];
        let stream_at = find(pdf, b"stream", dict_end).ok_or("no stream")?;
        // Skip the keyword plus its mandatory CR/LF terminator.
        let mut data_start = stream_at + "stream".len();
        if pdf[data_start..].starts_with(b"\r\n") {
            data_start += 2;
        } else if pdf[data_start..].starts_with(b"\n") {
            data_start += 1;
        }
        let data_end = find(pdf, b"endstream", data_start).ok_or("no endstream")?;
        let raw = &pdf[data_start..data_end];
        if dictionary.windows(b"/FlateDecode".len()).any(|w| w == b"/FlateDecode") {
            let mut inflated = Vec::new();
            flate2::read::ZlibDecoder::new(raw)
                .read_to_end(&mut inflated)
                .map_err(|e| format!("flate decode failed: {e}"))?;
            Ok(inflated)
        } else {
            Ok(raw.to_vec())
        }
    }
}

/// Content-stream interpreter: tracks the CTM stack, bakes the PDF
/// bottom-left origin into an SVG top-left one, and accumulates filled path
/// geometry into a single `d` attribute.
#[cfg(target_os = "macos")]
mod svg {
    use std::fmt::Write as _;

    use super::pdf::Page;

    type Matrix = [f64; 6]; // a b c d e f, applied as (a·x + c·y + e, b·x + d·y + f)

    fn multiply(m: Matrix, n: Matrix) -> Matrix {
        [
            m[0] * n[0] + m[2] * n[1],
            m[1] * n[0] + m[3] * n[1],
            m[0] * n[2] + m[2] * n[3],
            m[1] * n[2] + m[3] * n[3],
            m[0] * n[4] + m[2] * n[5] + m[4],
            m[1] * n[4] + m[3] * n[5] + m[5],
        ]
    }

    fn apply(m: Matrix, x: f64, y: f64) -> (f64, f64) {
        (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
    }

    /// Tracks the ink bounds of every emitted point so the final viewBox can
    /// hug the artwork instead of the (differently padded) symbol canvas.
    struct Bounds {
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
    }

    impl Bounds {
        fn new() -> Self {
            Self {
                min_x: f64::INFINITY,
                min_y: f64::INFINITY,
                max_x: f64::NEG_INFINITY,
                max_y: f64::NEG_INFINITY,
            }
        }

        fn include(&mut self, x: f64, y: f64) {
            self.min_x = self.min_x.min(x);
            self.min_y = self.min_y.min(y);
            self.max_x = self.max_x.max(x);
            self.max_y = self.max_y.max(y);
        }
    }

    fn push_point(d: &mut String, bounds: &mut Bounds, m: Matrix, x: f64, y: f64) {
        let (x, y) = apply(m, x, y);
        bounds.include(x, y);
        write!(d, "{x:.3} {y:.3}").expect("write point");
    }

    /// Extra padding around the artwork so antialiased edges are not clipped.
    const BLEED: f64 = 0.25;

    pub fn emit(page: &Page) -> Result<String, String> {
        // SVG's y axis points down: bake a flip into the initial CTM.
        // The final viewBox hugs the artwork (plus the bleed) so icons center
        // and scale uniformly wherever they are placed.
        let flip: Matrix = [1.0, 0.0, 0.0, -1.0, 0.0, page.height];
        let mut ctm = flip;
        let mut stack: Vec<Matrix> = Vec::new();
        let mut operands: Vec<f64> = Vec::new();
        let mut path = String::new();
        let mut bounds = Bounds::new();
        let mut even_odd = false;

        let content = String::from_utf8_lossy(&page.content);
        for token in content.split_whitespace() {
            if let Ok(number) = token.parse::<f64>() {
                operands.push(number);
                continue;
            }
            if token.starts_with('/') {
                continue; // color-space names etc. carry no geometry
            }
            let take = |operands: &mut Vec<f64>, n: usize| -> Result<Vec<f64>, String> {
                if operands.len() < n {
                    return Err(format!("operator {token} needs {n} operands"));
                }
                Ok(operands.split_off(operands.len() - n))
            };
            match token {
                "q" => stack.push(ctm),
                "Q" => ctm = stack.pop().ok_or("unbalanced Q")?,
                "cm" => {
                    let v = take(&mut operands, 6)?;
                    ctm = multiply(ctm, [v[0], v[1], v[2], v[3], v[4], v[5]]);
                }
                "m" => {
                    let v = take(&mut operands, 2)?;
                    path.push_str(" M ");
                    push_point(&mut path, &mut bounds, ctm, v[0], v[1]);
                }
                "l" => {
                    let v = take(&mut operands, 2)?;
                    path.push_str(" L ");
                    push_point(&mut path, &mut bounds, ctm, v[0], v[1]);
                }
                "c" => {
                    let v = take(&mut operands, 6)?;
                    path.push_str(" C ");
                    for pair in v.chunks_exact(2) {
                        push_point(&mut path, &mut bounds, ctm, pair[0], pair[1]);
                        path.push(' ');
                    }
                }
                "v" | "y" => {
                    // v: first control point equals the current point.
                    // y: second control point equals the end point. Neither
                    // appears in glyph fills; expand both to full cubics.
                    return Err(format!("operator {token} unsupported (report and extend)"));
                }
                "h" => path.push_str(" Z"),
                "re" => {
                    let rect = take(&mut operands, 4)?;
                    let (x, y, width, height) = (rect[0], rect[1], rect[2], rect[3]);
                    path.push_str(" M ");
                    push_point(&mut path, &mut bounds, ctm, x, y);
                    path.push_str(" L ");
                    push_point(&mut path, &mut bounds, ctm, x + width, y);
                    path.push_str(" L ");
                    push_point(&mut path, &mut bounds, ctm, x + width, y + height);
                    path.push_str(" L ");
                    push_point(&mut path, &mut bounds, ctm, x, y + height);
                    path.push_str(" Z");
                }
                "f*" => even_odd = true,
                "f" | "F" => {}
                // Clips, color setters, intents, and stroke-less ends carry
                // no geometry for filled glyphs.
                "W" | "W*" | "n" | "cs" | "CS" | "sc" | "SC" | "scn" | "SCN" | "rg" | "RG"
                | "g" | "G" | "ri" | "gs" | "w" | "J" | "j" | "M" | "d" | "i" => {
                    operands.clear();
                }
                other => {
                    return Err(format!("operator {other} unsupported (report and extend)"));
                }
            }
            // Fill/stroke/clip operators consume the operand stack.
            if matches!(token, "f" | "f*" | "F" | "W" | "W*" | "n" | "m" | "l" | "c" | "h" | "re")
            {
                operands.clear();
            }
        }
        if path.trim().is_empty() {
            return Err("content stream produced no path geometry".into());
        }

        let rule = if even_odd { "evenodd" } else { "nonzero" };
        // Icons ship monochrome; components recolor them through iced's svg
        // style color filter, so the intrinsic fill just needs to be opaque.
        let (min_x, min_y) = (bounds.min_x - BLEED, bounds.min_y - BLEED);
        let (width, height) =
            (bounds.max_x - bounds.min_x + 2.0 * BLEED, bounds.max_y - bounds.min_y + 2.0 * BLEED);
        Ok(format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{min_x:.3} {min_y:.3} {width:.3} {height:.3}\">\
             <path fill=\"black\" fill-rule=\"{rule}\" d=\"{path}\"/></svg>\n",
        ))
    }
}
