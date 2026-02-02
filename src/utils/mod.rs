use crate::prelude::*;

// API clients
pub mod maps;
pub mod openweathermap;

pub mod hex_slice;
pub use hex_slice::HexSlice;

pub fn to_sentence_case(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }

    let mut graphemes = s.graphemes(true);
    let mut cap = String::with_capacity(s.len());
    if let Some(first) = graphemes.next() {
        cap.push_str(&first.to_uppercase());
    }
    cap.push_str(graphemes.as_str());
    cap
}

pub fn clamp<T: PartialOrd>(input: T, min: T, max: T) -> T {
    debug_assert!(min <= max, "min must be less than or equal to max");
    if input < min {
        min
    } else if input > max {
        max
    } else {
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp() {
        // Test with integers
        assert_eq!(clamp(5, 0, 10), 5);
        assert_eq!(clamp(-5, 0, 10), 0);  // Below min
        assert_eq!(clamp(15, 0, 10), 10); // Above max
        assert_eq!(clamp(0, 0, 10), 0);   // At min
        assert_eq!(clamp(10, 0, 10), 10); // At max
        assert_eq!(clamp(5, 5, 5), 5);    // min == max

        // Test with floats
        assert_eq!(clamp(5.5, 0.0, 10.0), 5.5);
        assert_eq!(clamp(-1.5, 0.0, 10.0), 0.0);
        assert_eq!(clamp(11.5, 0.0, 10.0), 10.0);
    }
}
