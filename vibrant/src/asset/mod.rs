pub mod line;
pub mod scalar;

use line::LineSet;

pub struct Asset {
    pub line: Option<LineSet>,
}
