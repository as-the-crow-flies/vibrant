pub struct Setting<'a, T> {
    pub name: &'a str,
    pub value: T,
}

impl<'a> ToString for Setting<'a, u32> {
    fn to_string(&self) -> String {
        format!("const {}: u32 = {}u;\n", self.name, self.value)
    }
}
