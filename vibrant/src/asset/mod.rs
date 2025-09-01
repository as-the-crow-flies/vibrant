pub mod line;
pub mod texture;
pub mod utils;

use line::LineSet;

pub struct Asset {
    pub line: Option<LineSet>,
}
