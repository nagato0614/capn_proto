mod server;

// Cap’n Proto の自動生成ファイルを直接指定してモジュール化
#[path = "schema/hello_world_capnp.rs"]
mod hello_world_capnp;

fn main() {
    server::hello_world_server::write_to_stream().unwrap();
}
