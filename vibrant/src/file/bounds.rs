use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn scale(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn from_vertices<'a>(vertices: impl Iterator<Item = &'a Vec4>) -> Bounds {
        vertices
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

    pub fn from_bounds(bounds: &[Bounds]) -> Bounds {
        bounds.iter().fold(
            Bounds {
                min: Vec3::MAX,
                max: Vec3::MIN,
            },
            |result, bounds| Bounds {
                min: result.min.min(bounds.min),
                max: result.max.max(bounds.max),
            },
        )
    }

    pub fn transform(&self) -> Mat4 {
        Mat4::IDENTITY
            * Mat4::from_translation(self.min)
            * Mat4::from_scale(Vec3::splat(self.scale().max_element()))
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
