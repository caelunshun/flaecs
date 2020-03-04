#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) version: u32,
}

impl Entity {
    pub fn index(self) -> u32 {
        self.index
    }

    pub fn version(self) -> u32 {
        self.version
    }
}
