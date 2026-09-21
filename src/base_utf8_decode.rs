use std::collections::HashMap;

// Define the total number of padding options (1 to 14)
const MAX_PADDING: usize = 14;
lazy_static! {
    // Initialize the padding characters (using the first 14 Chinese characters)
    static ref PADDING_CHARS: Vec<char> = {
        let start = 0x4E00; // Unicode code point for the first Chinese character
    (0..MAX_PADDING)
        .map(|i| std::char::from_u32(start + i as u32).unwrap())
        .collect()
    };
    // Initialize the extended alphabet using Chinese characters
    static ref ALPHABET: Vec<char> = {
        // Starting from Unicode code point U+4E00 (CJK Unified Ideographs)
        let start = 0x4E00;
        let alphabet_size = 1 << 14; // 16384
        let mut vec = Vec::with_capacity(alphabet_size);
        for i in 0..alphabet_size {
            let code_point = start + i as u32;
            if let Some(ch) = std::char::from_u32(code_point) {
                vec.push(ch);
            } else {
                // If the code point is invalid, you may handle it accordingly.
                // For simplicity, we can skip invalid code points.
                continue;
            }
        }
        vec
    };
    // Create a reverse mapping from characters to indices for decoding
    static ref CHAR_TO_INDEX: HashMap<char, u32> = {
        let mut map = HashMap::with_capacity(1024);
        for (i, &ch) in ALPHABET.iter().enumerate() {
            map.insert(ch, i as u32);
        }
        map
    };
}

// Function to decode a string back into binary data using the Chinese character alphabet.
//
// The 14 padding markers are also ordinary alphabet characters (indices 0-13), so a
// string whose data happens to end with one of them is ambiguous. `encode` only
// appends a marker when the byte count leaves a partial 14-bit chunk, which means a
// genuine marker always leaves a multiple of 8 data bits; when reading the last
// character as a marker does not, it is data. The readings that stay ambiguous (a
// marker for 2 or 10 padding bits after a multiple of four characters) differ in
// length by two or three bytes, so a caller that knows the length it needs can
// resolve them with `decode_last_as_data`.
pub fn decode(s: &str) -> Vec<u8> {
    let chars = s.chars().collect::<Vec<char>>();
    match padding_bits_of(&chars) {
        Some(padding_bits) if ((chars.len() - 1) * 14 - padding_bits) % 8 == 0 => {
            decode_chars(&chars, Some(padding_bits))
        }
        _ => decode_chars(&chars, None),
    }
}

// Decode with the last character taken as data even when it is a padding marker.
pub fn decode_last_as_data(s: &str) -> Vec<u8> {
    let chars = s.chars().collect::<Vec<char>>();
    decode_chars(&chars, None)
}

// Whether the string ends in a padding marker and could be read either way.
pub fn ends_with_padding_marker(s: &str) -> bool {
    s.chars().last().is_some_and(|c| PADDING_CHARS.contains(&c))
}

// The number of padding bits the last character claims, if it is a padding marker.
fn padding_bits_of(chars: &[char]) -> Option<usize> {
    let last = *chars.last()?;
    PADDING_CHARS
        .iter()
        .position(|&c| c == last)
        .map(|padding_index| padding_index + 1) // padding_bits ranges from 1 to 14
}

fn decode_chars(chars: &[char], padding_bits: Option<usize>) -> Vec<u8> {
    let mut output = Vec::new();
    let mut bit_buffer: u128 = 0;
    let mut bit_buffer_len = 0;

    // With a padding marker the marker itself carries no data
    let data_len = match padding_bits {
        Some(_) => chars.len() - 1,
        None => chars.len(),
    };

    for (i, &ch) in chars.iter().enumerate().take(data_len) {
        let index = match CHAR_TO_INDEX.get(&ch) {
            Some(&idx) => idx as u128,
            None => {
                // Handle invalid characters
                continue;
            }
        };

        match padding_bits {
            Some(padding_bits) if i == data_len - 1 => {
                // Last character with padding
                let valid_bits = 14 - padding_bits;
                let data_bits = index >> padding_bits;
                bit_buffer = (bit_buffer << valid_bits) | data_bits;
                bit_buffer_len += valid_bits as u128;
            }
            _ => {
                bit_buffer = (bit_buffer << 14) | index;
                bit_buffer_len += 14;
            }
        }

        // Extract bytes from the bit buffer
        while bit_buffer_len >= 8 {
            bit_buffer_len -= 8;
            let byte = ((bit_buffer >> bit_buffer_len) & 0xFF) as u8;
            output.push(byte);
            bit_buffer &= (1 << bit_buffer_len) - 1;
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    // The encoder CloudDrive2 uses, kept here only to build test inputs
    fn encode(data: &[u8]) -> String {
        let mut output = String::new();
        let mut bit_buffer: u128 = 0;
        let mut bit_buffer_len = 0;
        for &byte in data {
            bit_buffer = (bit_buffer << 8) | byte as u128;
            bit_buffer_len += 8;
            while bit_buffer_len >= 14 {
                bit_buffer_len -= 14;
                output.push(ALPHABET[((bit_buffer >> bit_buffer_len) & 0x3FFF) as usize]);
                bit_buffer &= (1 << bit_buffer_len) - 1;
            }
        }
        if bit_buffer_len > 0 {
            let padding_bits = 14 - bit_buffer_len;
            output.push(ALPHABET[((bit_buffer << padding_bits) & 0x3FFF) as usize]);
            output.push(PADDING_CHARS[padding_bits - 1]);
        }
        output
    }

    #[test]
    fn round_trips_every_length() {
        let mut data = Vec::new();
        for i in 0..200u32 {
            assert_eq!(decode(&encode(&data)), data, "{} bytes", data.len());
            data.push((i * 37 % 251) as u8);
        }
    }

    // The last 14 bits of a 7-byte multiple can land on a padding marker
    // (indices 0-13); such a string has no marker of its own and must decode whole.
    // Two of the fourteen (2 and 10 padding bits) also read as a valid marker, and
    // there only the caller's length knowledge can tell; the rest decode directly.
    #[test]
    fn data_ending_in_a_padding_marker_is_not_padding() {
        for index in 0..MAX_PADDING as u8 {
            // 56 bytes = 32 chars exactly; the last char is the low 14 bits
            let mut data = vec![0xA5u8; 56];
            data[54] = 0;
            data[55] = index;
            let encoded = encode(&data);
            assert_eq!(encoded.chars().count(), 32);
            assert!(ends_with_padding_marker(&encoded));
            assert_eq!(decode_last_as_data(&encoded), data, "index {index}");
            let padding_bits = index as usize + 1;
            if padding_bits == 2 || padding_bits == 10 {
                assert_eq!(decode(&encoded).len(), 56 - (14 + padding_bits) / 8);
            } else {
                assert_eq!(decode(&encoded), data, "index {index}");
            }
        }
    }

    // 40 bytes really do end in a 2-bit marker after 24 characters; the marker
    // reading is right and the data reading is two bytes longer.
    #[test]
    fn ambiguous_marker_offers_both_readings() {
        let data = vec![0x5Au8; 40];
        let encoded = encode(&data);
        assert_eq!(encoded.chars().count(), 24);
        assert!(ends_with_padding_marker(&encoded));
        assert_eq!(decode(&encoded), data);
        assert_eq!(decode_last_as_data(&encoded).len(), 42);
    }
}
