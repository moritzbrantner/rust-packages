mod common;

fn main() {
    let metadata = common::PackageMetadata::current();
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--json") | Some("json") => println!("{}", metadata.json()),
        Some("help") | Some("--help") | Some("-h") => print_help(&metadata),
        Some("info") | None => print_info(&metadata),
        Some(command) => {
            eprintln!("unknown command `{command}`");
            print_help(&metadata);
            std::process::exit(2);
        }
    }
}

fn print_info(metadata: &common::PackageMetadata) {
    println!("package\t{}", metadata.name);
    println!("version\t{}", metadata.version);
    println!("description\t{}", metadata.description);
    println!("library\tuse {}", metadata.name.replace('-', "_"));
    println!(
        "api\tcargo run -p {} --bin {}-api",
        metadata.name, metadata.name
    );
    println!(
        "ui\tcargo run -p {} --bin {}-ui",
        metadata.name, metadata.name
    );
}

fn print_help(metadata: &common::PackageMetadata) {
    println!("{} package CLI", metadata.name);
    println!("commands:");
    println!("  info    print package surface metadata");
    println!("  json    print package surface metadata as JSON");
}
