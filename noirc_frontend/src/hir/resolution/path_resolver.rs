use super::import::{resolve_path_to_ns, ImportDirective, PathResolution};
use crate::Path;
use std::collections::HashMap;

use crate::graph::CrateId;
use crate::hir::def_map::{CrateDefMap, ModuleDefId, ModuleId};

pub trait PathResolver {
    fn resolve(&self, def_maps: &HashMap<CrateId, CrateDefMap>, path: Path) -> Option<ModuleDefId>;
}

pub struct FunctionPathResolver {
    // Module that we are resolving the path in
    module_id: ModuleId,
}

impl FunctionPathResolver {
    pub fn new(module_id: ModuleId) -> FunctionPathResolver {
        Self { module_id }
    }
}

impl PathResolver for FunctionPathResolver {
    fn resolve(&self, def_maps: &HashMap<CrateId, CrateDefMap>, path: Path) -> Option<ModuleDefId> {
        resolve_function_call_path(def_maps, self.module_id, path)
    }
}

// Resolve `foo::bar` in foo::bar::call() to the module with the function
pub fn resolve_function_call_path(
    def_maps: &HashMap<CrateId, CrateDefMap>,
    module_id: ModuleId,
    path: Path,
) -> Option<ModuleDefId> {
    // lets package up the path into an ImportDirective and resolve it using that
    let import = ImportDirective {
        module_id: module_id.local_id,
        path,
        alias: None,
    };

    let def_map = &def_maps[&module_id.krate];
    let path_res = resolve_path_to_ns(&import, def_map, def_maps);
    let ns = match path_res {
        // XXX: It is possible to propagate the actual segment of the resolver which failed
        PathResolution::Unresolved(seg) => return None,
        PathResolution::Resolved(ns) => ns,
    };

    // XXX: Note that we are returning the value and not a type.
    // In the future we can generalise and return a PerNs
    // Which the Resolver will then deal with
    // For now, since this is used only for function call paths, it is fine
    ns.take_values()
}
