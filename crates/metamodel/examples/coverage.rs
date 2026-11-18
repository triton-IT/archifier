//! Génère `docs/metamodel-coverage.md` à partir des profils embarqués.
//!
//! Usage : `cargo run -p archifier-metamodel --example coverage > docs/metamodel-coverage.md`

use archifier_metamodel::{coverage_report, MetamodelRegistry};

fn main() {
    match MetamodelRegistry::load_embedded() {
        Ok(registry) => print!("{}", coverage_report(&registry)),
        Err(error) => {
            eprintln!("Profils embarqués illisibles : {error}");
            std::process::exit(1);
        }
    }
}
