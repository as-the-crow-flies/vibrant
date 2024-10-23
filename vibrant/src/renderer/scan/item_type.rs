use crate::renderer::setting::Setting;

#[derive(Clone, Copy)]
pub enum ItemType {
    U32,
    I32,
    F32,
    UVEC2,
    UVEC4,
    IVEC2,
    IVEC4,
    FVEC2,
    FVEC4,
}

impl ItemType {
    pub fn size(&self) -> u32 {
        match self {
            ItemType::U32 => 4,
            ItemType::I32 => 4,
            ItemType::F32 => 4,
            ItemType::UVEC2 => 8,
            ItemType::UVEC4 => 16,
            ItemType::IVEC2 => 8,
            ItemType::IVEC4 => 16,
            ItemType::FVEC2 => 8,
            ItemType::FVEC4 => 16,
        }
    }
}

impl ToString for ItemType {
    fn to_string(&self) -> String {
        match self {
            ItemType::U32 => "u32",
            ItemType::I32 => "i32",
            ItemType::F32 => "f32",
            ItemType::UVEC2 => "vec2<u32>",
            ItemType::UVEC4 => "vec4<u32>",
            ItemType::IVEC2 => "vec2<i32>",
            ItemType::IVEC4 => "vec4<i32>",
            ItemType::FVEC2 => "vec2<f32>",
            ItemType::FVEC4 => "vec4<f32>",
        }
        .to_string()
    }
}

impl<'a> ToString for Setting<'a, ItemType> {
    fn to_string(&self) -> String {
        format!("alias {} = {};\n", self.name, self.value.to_string())
    }
}
