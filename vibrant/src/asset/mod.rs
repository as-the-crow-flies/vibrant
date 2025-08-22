pub mod line;
pub mod texture;

use line::LineSet;

pub struct Asset {
    pub line: Option<LineSet>,
}
