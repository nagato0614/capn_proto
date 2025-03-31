
fn main() -> Result<(), Box<dyn std::error::Error>> {
    capnpc::CompilerCommand::new()
        .src_prefix("src/schema")
        .file("src/schema/hello_world.capnp")
        .output_path("src/schema")
        .run()
        .expect("schema compiler command failed");
    Ok(())
}