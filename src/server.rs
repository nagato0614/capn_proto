#![allow(dead_code)]
use capnp::capability::Promise;
use capnp_rpc::pry;

#[path = "./schema/hello_world_capnp.rs"]
mod hello_world_capnp;

pub mod hello_world_server {
    use crate::server::hello_world_capnp::hello_world;
    use capnp::serialize_packed;

    pub fn write_to_stream() -> ::capnp::Result<()> {
        let mut message = ::capnp::message::Builder::new_default();
        let mut hello_world_builder = message.init_root::<hello_world::hello_request::Builder>();
        
        hello_world_builder.set_name("Hello, World!");

        serialize_packed::write_message(&mut ::std::io::stdout(), &message)
    }
}
