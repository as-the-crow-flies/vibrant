pub mod line;
pub mod texture;
pub mod transform;

use line::LineBuffer;
use transform::TransformBuffer;

#[derive(Default)]
pub struct Asset {
    pub line: Option<LineBuffer>,
    pub transform: Option<TransformBuffer>,
}
