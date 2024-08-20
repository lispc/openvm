use crate::BITS_PER_FE;

pub fn num_fe(num_bytes: usize) -> usize {
    ((num_bytes as f64 * 8.0) / BITS_PER_FE as f64).ceil() as usize
}

pub fn num_bytes(num_bits: usize) -> usize {
    (num_bits as f64 / 8.0).ceil() as usize
}
