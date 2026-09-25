//! Bitmap font for the LED text/marquee mode.
//!
//! Text is drawn UPRIGHT. The panel is 3 lanes wide and 13 rows tall, so a glyph
//! cell is 3 pixels wide (one pixel per lane) by N pixels tall. Letters therefore
//! stack down the panel and scroll upward.
//!
//! A glyph is stored as up to `GLYPH_MAX_H` rows, each a 3-bit row with bit 2 as
//! the leftmost pixel. Row 0 is the TOP of the glyph. `glyph_row()` returns rows
//! bottom-up so callers can index them directly against panel rows, where row 0
//! is the bottom of the fan.

/// Maximum glyph height. Taller letters let the font stay readable on a panel
/// that is 13 rows high.
pub const GLYPH_MAX_H: usize = 7;

/// Blank rows inserted between glyphs so characters stay legible.
pub const GLYPH_GAP: usize = 1;

/// One character's bitmap: `height` rows tall, 3 pixels wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    /// Rows from the TOP of the glyph downward. Bit 2 is the leftmost pixel.
    pub rows: [u8; GLYPH_MAX_H],
    /// Height in pixels, 1..=GLYPH_MAX_H.
    pub height: usize,
}

impl Glyph {
    const fn blank(height: usize) -> Self {
        Self {
            rows: [0; GLYPH_MAX_H],
            height,
        }
    }
}

/// Look up a character's bitmap.
///
/// 3 pixels wide is narrow, so letterforms are abstract by necessity. Where two
/// letters would otherwise be identical (O/0, I/1, S/5) the shapes are
/// deliberately differentiated.
pub fn glyph(ch: char) -> Glyph {
    let c = ch.to_ascii_uppercase();
    let g = |height: usize, rows: [u8; GLYPH_MAX_H]| Glyph { rows, height };

    match c {
        'A' => g(5, [0b010, 0b101, 0b111, 0b101, 0b101, 0, 0]),
        'B' => g(5, [0b110, 0b101, 0b110, 0b101, 0b110, 0, 0]),
        'C' => g(5, [0b011, 0b100, 0b100, 0b100, 0b011, 0, 0]),
        'D' => g(5, [0b110, 0b101, 0b101, 0b101, 0b110, 0, 0]),
        'E' => g(5, [0b111, 0b100, 0b110, 0b100, 0b111, 0, 0]),
        'F' => g(5, [0b111, 0b100, 0b110, 0b100, 0b100, 0, 0]),
        'G' => g(5, [0b011, 0b100, 0b101, 0b101, 0b011, 0, 0]),
        'H' => g(5, [0b101, 0b101, 0b111, 0b101, 0b101, 0, 0]),
        'I' => g(5, [0b111, 0b010, 0b010, 0b010, 0b111, 0, 0]),
        'J' => g(5, [0b001, 0b001, 0b001, 0b101, 0b111, 0, 0]),
        'K' => g(5, [0b101, 0b101, 0b110, 0b101, 0b101, 0, 0]),
        'L' => g(5, [0b100, 0b100, 0b100, 0b100, 0b111, 0, 0]),
        'M' => g(5, [0b101, 0b111, 0b101, 0b101, 0b101, 0, 0]),
        'N' => g(5, [0b101, 0b111, 0b111, 0b101, 0b101, 0, 0]),
        'O' => g(5, [0b111, 0b101, 0b101, 0b101, 0b111, 0, 0]),
        'P' => g(5, [0b110, 0b101, 0b110, 0b100, 0b100, 0, 0]),
        'Q' => g(5, [0b111, 0b101, 0b101, 0b111, 0b001, 0, 0]),
        'R' => g(5, [0b110, 0b101, 0b110, 0b101, 0b101, 0, 0]),
        'S' => g(5, [0b011, 0b100, 0b010, 0b001, 0b110, 0, 0]),
        'T' => g(5, [0b111, 0b010, 0b010, 0b010, 0b010, 0, 0]),
        'U' => g(5, [0b101, 0b101, 0b101, 0b101, 0b111, 0, 0]),
        'V' => g(5, [0b101, 0b101, 0b101, 0b101, 0b010, 0, 0]),
        'W' => g(5, [0b101, 0b101, 0b111, 0b111, 0b101, 0, 0]),
        'X' => g(5, [0b101, 0b101, 0b010, 0b101, 0b101, 0, 0]),
        'Y' => g(5, [0b101, 0b101, 0b010, 0b010, 0b010, 0, 0]),
        'Z' => g(5, [0b111, 0b001, 0b010, 0b100, 0b111, 0, 0]),

        // Digits are 7 tall so they are clearly distinct from letters.
        '0' => g(7, [0b111, 0b101, 0b101, 0b101, 0b101, 0b101, 0b111]),
        '1' => g(7, [0b010, 0b110, 0b010, 0b010, 0b010, 0b010, 0b111]),
        '2' => g(7, [0b111, 0b001, 0b001, 0b111, 0b100, 0b100, 0b111]),
        '3' => g(7, [0b111, 0b001, 0b001, 0b111, 0b001, 0b001, 0b111]),
        '4' => g(7, [0b101, 0b101, 0b101, 0b111, 0b001, 0b001, 0b001]),
        '5' => g(7, [0b111, 0b100, 0b100, 0b111, 0b001, 0b001, 0b111]),
        '6' => g(7, [0b111, 0b100, 0b100, 0b111, 0b101, 0b101, 0b111]),
        '7' => g(7, [0b111, 0b001, 0b001, 0b010, 0b010, 0b010, 0b010]),
        '8' => g(7, [0b111, 0b101, 0b101, 0b111, 0b101, 0b101, 0b111]),
        '9' => g(7, [0b111, 0b101, 0b101, 0b111, 0b001, 0b001, 0b111]),

        ' ' => Glyph::blank(3),
        '-' => g(3, [0b000, 0b111, 0b000, 0, 0, 0, 0]),
        '_' => g(3, [0b000, 0b000, 0b111, 0, 0, 0, 0]),
        '.' => g(3, [0b000, 0b000, 0b010, 0, 0, 0, 0]),
        ',' => g(3, [0b000, 0b010, 0b100, 0, 0, 0, 0]),
        '!' => g(5, [0b010, 0b010, 0b010, 0b000, 0b010, 0, 0]),
        '?' => g(5, [0b111, 0b001, 0b011, 0b000, 0b010, 0, 0]),
        ':' => g(3, [0b000, 0b010, 0b000, 0, 0, 0, 0]),
        '+' => g(5, [0b000, 0b010, 0b111, 0b010, 0b000, 0, 0]),
        '=' => g(5, [0b000, 0b111, 0b000, 0b111, 0b000, 0, 0]),
        '/' => g(5, [0b001, 0b001, 0b010, 0b100, 0b100, 0, 0]),
        '(' => g(5, [0b001, 0b010, 0b100, 0b010, 0b001, 0, 0]),
        ')' => g(5, [0b100, 0b010, 0b001, 0b010, 0b100, 0, 0]),
        '\'' => g(3, [0b010, 0b010, 0b000, 0, 0, 0, 0]),

        // Anything else renders as a space rather than a wall of noise.
        _ => Glyph::blank(3),
    }
}

/// Reverse the three pixel bits of a glyph row.
///
/// Glyph bitmaps are written with bit 2 as the LEFTMOST pixel, because that is how
/// a 3-wide row reads in source. But the engine indexes lanes with
/// `(row >> lane) & 1`, where lane 0 must be the leftmost pixel. Without this
/// reversal the left and right pixels swap and the letters come out mirrored.
const fn mirror3(row: u8) -> u8 {
    ((row & 0b001) << 2) | (row & 0b010) | ((row & 0b100) >> 2)
}

/// Return a glyph's rows ordered BOTTOM-UP and in lane order.
///
/// Panel row 0 is the bottom of the fan, so callers index the result directly
/// against panel rows. Bits are returned so that bit 0 is lane 0 - the leftmost
/// pixel - matching how the engine reads them.
pub fn glyph_rows_bottom_up(ch: char) -> Vec<u8> {
    let gl = glyph(ch);
    let h = gl.height.clamp(1, GLYPH_MAX_H);
    // Stored top-down and with bit 2 leftmost: reverse for row order, mirror for
    // lane order.
    gl.rows[..h]
        .iter()
        .rev()
        .map(|r| mirror3(*r))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_fit_three_pixels_wide() {
        for c in "ABCXYZ019!?.- _+/'=,:()".chars() {
            let gl = glyph(c);
            for (i, row) in gl.rows.iter().enumerate() {
                assert!(row & !0b111 == 0, "{c} row {i} exceeds 3 pixels: {row:#05b}");
            }
        }
    }

    #[test]
    fn glyph_heights_are_in_range() {
        for c in "ABCXYZ019!?.- _+".chars() {
            let h = glyph(c).height;
            assert!(h >= 1 && h <= GLYPH_MAX_H, "{c} has height {h}");
        }
    }

    #[test]
    fn unused_rows_are_blank() {
        for c in "ABCXYZ019".chars() {
            let gl = glyph(c);
            for row in &gl.rows[gl.height..] {
                assert_eq!(*row, 0, "{c} has ink outside its declared height");
            }
        }
    }

    #[test]
    fn bottom_up_reverses_the_top_row() {
        // 'T' has a solid top bar and a centred stem. Bottom-up, the LAST entry
        // is the top bar. Bit 0 must be the leftmost pixel, so a symmetrical row
        // like the crossbar is unchanged by mirroring, and the stem is centred.
        let rows = glyph_rows_bottom_up('T');
        assert_eq!(*rows.last().unwrap(), 0b111); // top bar
        assert_eq!(*rows.first().unwrap(), 0b010); // stem at the bottom
    }

    #[test]
    fn lane_bits_are_left_to_right() {
        // 'L' is the clearest asymmetry test: a stem down the LEFT with a foot
        // extending right at the bottom. If lane bits were mirrored, the stem
        // would appear on the right and the letter would read backwards.
        let rows = glyph_rows_bottom_up('L');
        let top = *rows.last().unwrap();
        let bottom = rows[0];

        // Bit 0 is lane 0, the leftmost pixel, so the stem must set bit 0 only.
        assert_eq!(top, 0b001, "L's stem must be in lane 0 (leftmost)");
        assert_eq!(bottom, 0b111, "L's foot must span all three lanes");
    }

    #[test]
    fn mirror3_swaps_the_outer_bits() {
        assert_eq!(mirror3(0b100), 0b001);
        assert_eq!(mirror3(0b001), 0b100);
        assert_eq!(mirror3(0b010), 0b010);
        assert_eq!(mirror3(0b101), 0b101);
    }

    #[test]
    fn bottom_up_length_matches_height() {
        for c in "AI09".chars() {
            assert_eq!(glyph_rows_bottom_up(c).len(), glyph(c).height);
        }
    }

    #[test]
    fn o_and_zero_are_distinguishable() {
        // 3 pixels wide is tight, so guard the obvious collisions.
        assert_ne!(glyph('O').rows, glyph('0').rows);
    }
}
