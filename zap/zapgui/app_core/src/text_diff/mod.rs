pub mod moves;
pub mod pair;
pub mod post_process;
pub mod rows;
pub mod similarity;

pub use moves::{MoveRole, MoveSpan};
pub use pair::{pair_lines, pair_lines_full};
pub use post_process::{debug_format, process, DiffRowProcessedForUi};
pub use rows::{DiffRow, RowKind, Side};
pub use similarity::{dice_over_tokens, similarity, tokenize, weighted_lcs, SIMILARITY_THRESHOLD};