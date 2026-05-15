mod common;

fn main() -> std::io::Result<()> {
    common::run_server(common::package_json_response)
}
