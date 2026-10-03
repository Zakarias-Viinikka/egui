pub mod pair;
pub mod post_process;
pub mod rows;

pub use pair::pair_lines;
pub use post_process::{debug_format, process, DiffRowProcessedForUi};
pub use rows::{DiffRow, RowKind, Side};
