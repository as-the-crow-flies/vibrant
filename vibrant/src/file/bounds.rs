use glam::{Vec3, Vec4, Vec4Swizzles};

#[derive(Debug)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn scale(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn from_vertices(vertices: &[Vec4]) -> Bounds {
        vertices
            .iter()
            .filter(|&vertex| vertex.is_finite())
            .fold(
                Bounds {
                    min: Vec3::MAX,
                    max: Vec3::MIN,
                },
                |bounds, vertex| Bounds {
                    min: bounds.min.min(vertex.xyz()),
                    max: bounds.max.max(vertex.xyz()),
                },
            )
            .expand()
    }

    fn expand(self) -> Self {
        Bounds {
            min: self.min - self.scale() * 0.05,
            max: self.max + self.scale() * 0.05,
        }
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            min: Vec3::MAX,
            max: Vec3::MIN,
        }
    }
}
