
fn main() -> Result<(), Box<dyn std::error::Error>> {
    capnpc::CompilerCommand::new()
        .src_prefix("src/schema") // スキーマがあるディレクトリ
        .file("src/schema/hello_world.capnp")// スキーマファイル
        .output_path("src/schema")// 出力先ディレクトリ
        .run()
        .expect("schema compiler command failed");
    Ok(())
}