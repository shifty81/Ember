use ember_validation::validate_architecture;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("architecture");
    let root = args.get(1).map(String::as_str).unwrap_or(".");
    let report = match command {
        "architecture" => validate_architecture(Path::new(root)),
        other => {
            eprintln!("unknown Ember validation command: {other}");
            std::process::exit(2);
        }
    };
    for diagnostic in &report.diagnostics {
        println!(
            "{:?} {}: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        );
    }
    if report.has_errors() {
        eprintln!(
            "Ember architecture validation failed with {} diagnostic(s)",
            report.diagnostics.len()
        );
        std::process::exit(1);
    }
    println!("Ember architecture validation PASS");
}
