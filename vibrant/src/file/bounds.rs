use glam::Vec3;

#[derive(Debug)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn scale(&self) -> f32 {
        2.0 * self.min.abs().max_element().max(self.max.max_element())
    }

    pub fn from_vertices(vertices: &[Vec3]) -> Bounds {
        vertices
            .iter()
            .filter(|&vertex| vertex.is_finite())
            .fold(
                Bounds {
                    min: Vec3::MAX,
                    max: Vec3::MIN,
                },
                |bounds, vertex| Bounds {
                    min: bounds.min.min(*vertex),
                    max: bounds.max.max(*vertex),
                },
            )
            .grow(1.01)
    }

    pub fn grow(&self, factor: f32) -> Bounds {
        Bounds {
            min: self.min * factor,
            max: self.max * factor,
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
