use std::collections::HashMap;

use fm::FileId;

use super::{ItemScope, LocalModuleId};

#[derive(Default, Debug, PartialEq, Eq)]
pub struct ModuleData {
    pub parent: Option<LocalModuleId>,
    pub children: HashMap<String, LocalModuleId>,
    pub scope: ItemScope,

    pub origin: ModuleOrigin,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ModuleOrigin {
    CrateRoot(FileId),
    File(FileId),
}

impl Into<FileId> for ModuleOrigin {
    fn into(self) -> FileId {
        match self {
            ModuleOrigin::CrateRoot(file_id) => file_id,
            ModuleOrigin::File(file_id) => file_id,
        }
    }
}

impl Default for ModuleOrigin {
    fn default() -> Self {
        ModuleOrigin::CrateRoot(FileId::default())
    }
}
