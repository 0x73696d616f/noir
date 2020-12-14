
pub mod module_disc;
mod file_manager;

use std::path::{Path, PathBuf};
use noirc_errors::{FileMap, Reporter, FileID};
use noirc_errors::DiagnosableError;
use noirc_frontend::lexer::Lexer;
use noirc_frontend::parser::{Parser, Program};
use noir_evaluator::{Evaluator};
use noirc_frontend::analyser;
use noirc_frontend::ast::Type;
use acir::circuit::Circuit;
use crate::module_disc::ModType;
use nargo::{CrateManager, CrateUnit, crate_unit::CrateType};
pub use nargo::dir_util;

pub struct Driver {
    file_map : FileMap,
}

pub struct CompiledProgram {
    pub circuit : Circuit,
    pub num_witnesses : usize, 
    pub num_public_inputs : usize, 
    pub abi : Option<Vec<(String, Type)>>
}

impl Driver{
    pub fn new() -> Self {
        Driver {
            file_map : FileMap::new(),
        }
    }

    // This is here for compatibility.
    pub fn compile_file(&mut self, root_file : PathBuf, source: String) -> CompiledProgram {
        self.compile_crate(root_file)
    }

    pub fn compile_crate<P: AsRef<Path>>(&mut self, root_file : P) -> CompiledProgram {

        let root_dir = root_file.as_ref().parent().unwrap();

        use crate::dir_util::DirType;

        let mut driver = Driver::new();
        let mut module_system = CrateUnit::new(root_dir.to_path_buf(), CrateType::LIBRARY);
        
        let bin_dir = DirType::Binary;
        let lib_dir = DirType::Library;
        
        let is_bin_present = bin_dir.file_path(&root_dir).is_some();
        let is_lib_present = lib_dir.file_path(&root_dir).is_some();
        
        if !is_lib_present {
            println!("A project must contain a library file");
            std::process::exit(1);   
        }
        
        let lib_path =  lib_dir.file_path(&root_dir).unwrap();
        
        module_disc::recursively_parse(&mut driver, &mut module_system, ModType::SubModule(lib_path)); 
    
        let mut crate_manager = CrateManager::with_local_crate(module_system);

        let mut abi = None;

        if is_bin_present {
            let main_path =  bin_dir.file_path(&root_dir).unwrap();
            let file_as_string = std::fs::read_to_string(&main_path).unwrap();
            let (program, _) = driver.parse_file(&main_path, file_as_string);

        
            let mut krate = CrateUnit::new(root_dir.clone().into(), CrateType::BINARY);
            krate.insert_module(main_path, "main".to_owned(), program.clone());
            crate_manager.insert_crate("main".to_owned(), krate);

            abi = program.abi();
            
            assert!(program.module_decls.is_empty(), "main file cannot contain module declarations");
        }

        // Add std_lib
        driver.load_low_level_libraries(&mut crate_manager);    


        // Analysis Phase        
        analyser::check_crates(&mut crate_manager).unwrap();
        //XXX: Currently, we do not propagate the file_id with the crate, so error reporting cannot be accurate
        // if let Err(errs) =  analyser::check_crates(&mut crate_manager) {
        //     let diagnostics : Vec<_> = errs.into_iter().map(|err| err.to_diagnostic()).collect();
        //     Reporter::with_diagnostics(file_id, &self.file_map, &diagnostics);
        //     std::process::exit(1);
        // }

        let evaluator = Evaluator::new(crate_manager).expect("None was returned from Evaluator constructor. Expected a main file, libraries only are not supported at the moment ");
    
        let (circuit, num_witnesses, num_public_inputs) = evaluator.evaluate();

        CompiledProgram {
            circuit,
            num_witnesses,
            num_public_inputs,
            abi
        }
    }

    pub fn parse_file(&mut self, file_name : &PathBuf, source: String) -> (Program, FileID){ 
                // First add the file to the system 
                let file_id = self.file_map.add_file(file_name.into(), source);

                let file = self.file_map.get_file(file_id).unwrap();
        
                // Lex the file using it's file id to generate error diagnostics (span)
                let lexer = Lexer::from_file(file_id.0, file);
        
                let mut parser = Parser::new(lexer);
        
                let program = match parser.parse_program() {
                    Ok(program) => program,
                    Err(errs) => {
                        let diagnostics : Vec<_> = errs.into_iter().map(|err| err.to_diagnostic()).collect();
                        // Parse errors and print diagnostic for them
                        Reporter::with_diagnostics(file_id, &self.file_map, &diagnostics);
                        std::process::exit(1);            
                    }
                };

                (program, file_id)
    }

    pub fn load_low_level_libraries(&mut self, crate_manager : &mut CrateManager<Program>) {

        let path_to_std_lib_file = path_to_stdlib().join("lib.nr");

        let mut krate = CrateUnit::new(path_to_stdlib(), CrateType::LIBRARY);

        module_disc::recursively_parse(self, &mut krate,ModType::SubModule(path_to_std_lib_file));
   
        crate_manager.insert_crate("std".to_owned(), krate);
    }
}

fn path_to_stdlib() -> PathBuf {
    dirs::config_dir().unwrap().join("noir-lang").join("std_lib")
}