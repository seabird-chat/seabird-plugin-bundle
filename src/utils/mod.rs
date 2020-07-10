use crate::prelude::*;

// API clients
pub mod maps;
pub mod openweathermap;

pub mod hex_slice;
pub use hex_slice::HexSlice;

pub fn to_sentence_case(s: &str) -> String {
    let mut graphemes = s.graphemes(true);
    let mut cap = String::with_capacity(s.len());
    cap.push_str(&graphemes.next().unwrap().to_uppercase());
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
