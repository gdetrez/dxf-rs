use std::borrow::Cow;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

/// Contains the data portion of a `CodePair`.
#[derive(PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub enum CodePairValue {
    Boolean(i16),
    Integer(i32),
    Long(i64),
    Short(i16),
    Double(f64),
    Str(String),
}

// internal visibility only
impl CodePairValue {
    pub(crate) fn un_escape_string(val: &'_ str) -> Cow<'_, str> {
        fn needs_un_escaping(c: char) -> bool {
            c == '^'
        }

        if let Some(first) = val.find(needs_un_escaping) {
            let mut result = String::from(&val[0..first]);
            result.reserve(val.len() - first);
            let rest = val[first..].chars();
            let mut do_escape = false;
            for c in rest {
                match c {
                    '^' if !do_escape => do_escape = true,
                    _ => {
                        if do_escape {
                            do_escape = false;
                            let c = match c {
                                '@' => 0x00,
                                'A' => 0x01,
                                'B' => 0x02,
                                'C' => 0x03,
                                'D' => 0x04,
                                'E' => 0x05,
                                'F' => 0x06,
                                'G' => 0x07,
                                'H' => 0x08,
                                'I' => 0x09,
                                'J' => 0x0A,
                                'K' => 0x0B,
                                'L' => 0x0C,
                                'M' => 0x0D,
                                'N' => 0x0E,
                                'O' => 0x0F,
                                'P' => 0x10,
                                'Q' => 0x11,
                                'R' => 0x12,
                                'S' => 0x13,
                                'T' => 0x14,
                                'U' => 0x15,
                                'V' => 0x16,
                                'W' => 0x17,
                                'X' => 0x18,
                                'Y' => 0x19,
                                'Z' => 0x1A,
                                '[' => 0x1B,
                                '\\' => 0x1C,
                                ']' => 0x1D,
                                '^' => 0x1E,
                                '_' => 0x1F,
                                ' ' => b'^',
                                _ => c as u8, // invalid escape sequence, just keep the character
                            };

                            result.push(c as char);
                        } else {
                            result.push(c);
                        }
                    }
                }
            }

            result.into()
        } else {
            val.into()
        }
    }
}

impl Clone for CodePairValue {
    fn clone(&self) -> Self {
        match self {
            CodePairValue::Boolean(b) => CodePairValue::Boolean(*b),
            CodePairValue::Integer(i) => CodePairValue::Integer(*i),
            CodePairValue::Long(l) => CodePairValue::Long(*l),
            CodePairValue::Short(s) => CodePairValue::Short(*s),
            CodePairValue::Double(d) => CodePairValue::Double(*d),
            CodePairValue::Str(ref s) => CodePairValue::Str(String::from(s.as_str())),
        }
    }
}

impl Debug for CodePairValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CodePairValue::Boolean(s) => write!(f, "{}", s),
            CodePairValue::Integer(i) => write!(f, "{: >9}", i),
            CodePairValue::Long(l) => write!(f, "{}", l),
            CodePairValue::Short(s) => write!(f, "{: >6}", s),
            CodePairValue::Double(d) => write!(f, "{}", format_f64(*d)),
            CodePairValue::Str(ref s) => write!(f, "{}", s),
        }
    }
}

impl Display for CodePairValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self) // fall back to debug
    }
}

pub(crate) fn escape_control_characters(val: &str) -> String {
    fn needs_escaping(c: char) -> bool {
        let c = c as u8;
        c <= 0x1F || c == 0x5E
    }

    let mut result = String::from("");
    for c in val.chars() {
        if needs_escaping(c) {
            result.push('^');
            match c as u8 {
                0x00 => result.push('@'),
                0x01 => result.push('A'),
                0x02 => result.push('B'),
                0x03 => result.push('C'),
                0x04 => result.push('D'),
                0x05 => result.push('E'),
                0x06 => result.push('F'),
                0x07 => result.push('G'),
                0x08 => result.push('H'),
                0x09 => result.push('I'),
                0x0A => result.push('J'),
                0x0B => result.push('K'),
                0x0C => result.push('L'),
                0x0D => result.push('M'),
                0x0E => result.push('N'),
                0x0F => result.push('O'),
                0x10 => result.push('P'),
                0x11 => result.push('Q'),
                0x12 => result.push('R'),
                0x13 => result.push('S'),
                0x14 => result.push('T'),
                0x15 => result.push('U'),
                0x16 => result.push('V'),
                0x17 => result.push('W'),
                0x18 => result.push('X'),
                0x19 => result.push('Y'),
                0x1A => result.push('Z'),
                0x1B => result.push('['),
                0x1C => result.push('\\'),
                0x1D => result.push(']'),
                0x1E => result.push('^'),
                0x1F => result.push('_'),
                0x5E => result.push(' '),
                _ => panic!("this should never happen"),
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[test]
fn test_escape_control_characters() {
    assert_eq!("a^G^ ^^ b", escape_control_characters("a\u{7}^\u{1E} b"));
}

pub(crate) fn escape_unicode_to_ascii(val: &str) -> String {
    let mut result = String::from("");

    for c in val.chars() {
        let b = c as u32;
        if b >= 128 {
            result.push_str(&format!("\\U+{:04X}", b));
        } else {
            result.push(c);
        }
    }

    result
}

#[test]
fn test_unicode_escape() {
    // values in the middle of a string
    assert_eq!(
        "Rep\\U+00E8re pi\\U+00E8ce",
        escape_unicode_to_ascii("Repère pièce")
    );

    // value is the entire string
    assert_eq!("\\U+4F60\\U+597D", escape_unicode_to_ascii("你好"));
}

pub(crate) fn un_escape_ascii_to_unicode(val: &str) -> String {
    let mut result = String::from("");
    let mut seq = String::from("");
    let mut in_escape_sequence = false;
    let mut sequence_start = 0;

    for (i, c) in val.chars().enumerate() {
        if !in_escape_sequence {
            if c == '\\' {
                in_escape_sequence = true;
                sequence_start = i;
                seq.clear();
                seq.push(c);
            } else {
                result.push(c);
            }
        } else {
            seq.push(c);
            if i == sequence_start + 6 {
                in_escape_sequence = false;
                if seq.starts_with("\\U+") {
                    let code_str = &seq[3..];
                    let decoded = match u32::from_str_radix(code_str, 16) {
                        Ok(code) => match std::char::from_u32(code) {
                            Some(c) => c,
                            None => '?',
                        },
                        Err(_) => '?',
                    };
                    result.push(decoded);
                } else {
                    result.push_str(&seq);
                }

                seq.clear();
            }
        }
    }

    result
}

#[test]
fn test_ascii_unescape() {
    // values in the middle of the string
    assert_eq!(
        "Repère pièce",
        un_escape_ascii_to_unicode("Rep\\U+00E8re pi\\U+00E8ce")
    );

    // value is entire string
    assert_eq!("你好", un_escape_ascii_to_unicode("\\U+4F60\\U+597D"));
}

/// Formats an `f64` value with up to 12 digits of precision, ensuring at least one trailing digit after the decimal.
fn format_f64(val: f64) -> String {
    // format with 12 digits of precision
    let mut val = format!("{:.12}", val);

    // trim trailing zeros
    while val.ends_with('0') {
        val.pop();
    }

    // ensure it doesn't end with a decimal
    if val.ends_with('.') {
        val.push('0');
    }

    val
}
