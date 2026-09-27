#[derive(Debug)]
pub enum UnescapeError {
    #[allow(dead_code)]
    InvalidEscape(char),
    InvalidUnicodeEscape,
    InvalidSurrogatePair,
    UnescapedControlCharacter,
}

pub const ESCAPE_TOKENS: [&str; 8] = ["\\\"", "\\\\", "\\/", "\\b", "\\f", "\\n", "\\r", "\\t"];
pub const MUST_BE_ESCAPED: [&str; 34] = [
    "\"", "\\", "\u{0000}", "\u{0001}", "\u{0002}", "\u{0003}", "\u{0004}", "\u{0005}", "\u{0006}",
    "\u{0007}", "\u{0008}", "\u{0009}", "\u{000A}", "\u{000B}", "\u{000C}", "\u{000D}", "\u{000E}",
    "\u{000F}", "\u{0010}", "\u{0011}", "\u{0012}", "\u{0013}", "\u{0014}", "\u{0015}", "\u{0016}",
    "\u{0017}", "\u{0018}", "\u{0019}", "\u{001A}", "\u{001B}", "\u{001C}", "\u{001D}", "\u{001E}",
    "\u{001F}",
];

pub fn unescape_json_string(input: &str) -> Result<String, UnescapeError> {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            // JSON không cho phép control character literal trong string.
            if (ch as u32) < 0x20 {
                return Err(UnescapeError::UnescapedControlCharacter);
            }

            output.push(ch);
            continue;
        }

        let escape = chars.next().ok_or(UnescapeError::InvalidEscape('\\'))?;

        match escape {
            '"' => output.push('"'),
            '\\' => output.push('\\'),
            '/' => output.push('/'),
            'b' => output.push('\u{0008}'),
            'f' => output.push('\u{000C}'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),

            'u' => {
                let first = read_hex_value(&mut chars)?;

                if is_high_surrogate(first) {
                    // High surrogate must be followed by \uXXXX
                    match (chars.next(), chars.next()) {
                        (Some('\\'), Some('u')) => {}
                        _ => return Err(UnescapeError::InvalidSurrogatePair),
                    }

                    let second = read_hex_value(&mut chars)?;

                    if !is_low_surrogate(second) {
                        return Err(UnescapeError::InvalidSurrogatePair);
                    }

                    let code_point =
                        0x1_0000 + (((first as u32) - 0xD800) << 10) + ((second as u32) - 0xDC00);

                    let c =
                        char::from_u32(code_point).ok_or(UnescapeError::InvalidUnicodeEscape)?;

                    output.push(c);
                } else if is_low_surrogate(first) {
                    // Rust String không thể chứa một surrogate đơn lẻ.
                    return Err(UnescapeError::InvalidSurrogatePair);
                } else {
                    let c =
                        char::from_u32(first as u32).ok_or(UnescapeError::InvalidUnicodeEscape)?;

                    output.push(c);
                }
            }

            other => return Err(UnescapeError::InvalidEscape(other)),
        }
    }

    Ok(output)
}

fn is_high_surrogate(value: u16) -> bool {
    (0xD800..=0xDBFF).contains(&value)
}

fn is_low_surrogate(value: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&value)
}

fn read_hex_value<I>(chars: &mut I) -> Result<u16, UnescapeError>
where
    I: Iterator<Item = char>,
{
    let mut value = 0u16;

    for _ in 0..4 {
        let ch = chars.next().ok_or(UnescapeError::InvalidUnicodeEscape)?;

        let digit = ch.to_digit(16).ok_or(UnescapeError::InvalidUnicodeEscape)?;

        value = (value << 4) | digit as u16;
    }

    Ok(value)
}
