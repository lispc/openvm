pub mod committed_page;
pub mod exec;
pub mod expr;
pub mod node;
pub mod utils;

/// Number of columns, from the left side of the Schema, that are index columns. Keep in mind that you
/// will also need to change the underlying `Page` data's idx cols to match this.
pub const NUM_IDX_COLS: usize = 1;

pub const BITS_PER_FE: usize = 16;
pub const MAX_ROWS: usize = 64;
pub const PCS_LOG_DEGREE: usize = 16;
pub const RANGE_CHECK_BITS: usize = 16;

pub const PAGE_BUS_IDX: usize = 0;
pub const RANGE_BUS_IDX: usize = 1;
pub const OPS_BUS_IDX: usize = 2;

const _: () = assert!(BITS_PER_FE < 31, "BITS_PER_FE must be less than 31");
