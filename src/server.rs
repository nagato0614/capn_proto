#![allow(dead_code)]

use capnp::capability::Promise;
use capnp_rpc::pry;

// hello_world_capnp を使う（main.rsで #[path] で定義済みなので使える）
use crate::hello_world_capnp::hello_world;
use capnp::serialize_packed;

pub mod hello_world_server {
    use super::*;

    pub fn write_to_stream() -> ::capnp::Result<()> {
        let mut message = ::capnp::message::Builder::new_default();

        // バージョンが新しければ hello_request::Builder になる
        let mut hello_world_builder = message.init_root::<hello_world::hello_request::Builder>();

        hello_world_builder.set_name("Hello, World!");

        serialize_packed::write_message(&mut ::std::io::stdout(), &message)
    }
}
