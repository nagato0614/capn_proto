use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::net::UnixListener;
use tokio::time::{sleep, Duration};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::hello_world_capnp::hello_world;

// サーバー側で実装する HelloWorld のロジック
pub struct HelloWorldImpl;

// サーバーが保持するイベントリスナー（クライアントから渡される）
static mut EVENT_LISTENER: Option<hello_world::event_listener::Client> = None;

// Cap’n Proto の RPC サーバー側の処理を実装
impl hello_world::Server for HelloWorldImpl {
    fn say_hello(
        &mut self,
        params: hello_world::SayHelloParams,
        mut results: hello_world::SayHelloResults,
    ) -> Promise<(), capnp::Error> {
        let request = pry!(pry!(params.get()).get_request());
        let name = pry!(request.get_name());
        let value = request.get_value();

        println!("[server] name='{:?}', value={}", name, value);

        let mut reply = results.get().init_reply();
        let message = format!("Hello {:?}, I got {}", name, value);
        reply.set_message(&message);

        Promise::ok(())
    }

    fn subscribe_events(
        &mut self,
        params: hello_world::SubscribeEventsParams,
        _results: hello_world::SubscribeEventsResults,
    ) -> Promise<(), capnp::Error> {
        // クライアントから送られてきた listener を保持
        let listener = pry!(pry!(params.get()).get_listener());

        unsafe {
            EVENT_LISTENER = Some(listener.clone());
        }

        println!("[server] イベントリスナー登録完了！");

        // 非同期で通知を送るタスクを起動
        tokio::task::spawn_local(async move {
            loop {
                sleep(Duration::from_secs(5)).await;

                unsafe {
                    if let Some(ref listener) = EVENT_LISTENER {
                        let mut request = listener.on_event_request();
                        request.get().set_message("サーバーからの定期通知です！");

                        // エラーはログに出すがループは継続
                        if let Err(e) = request.send().promise.await {
                            eprintln!("[server] イベント送信失敗: {:?}", e);
                        } else {
                            println!("[server] イベント送信成功！");
                        }
                    }
                }
            }
        });

        Promise::ok(())
    }
}

// サーバーを起動する関数
pub async fn run_server(socket_path: &str) -> anyhow::Result<()> {
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;

    loop {
        let (stream, _) = listener.accept().await?;
        let (read_half, write_half) = stream.into_split();
        let read_half = read_half.compat();
        let write_half = write_half.compat_write();

        let network = twoparty::VatNetwork::new(
            read_half,
            write_half,
            rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );

        let client = capnp_rpc::new_client::<hello_world::Client, _>(HelloWorldImpl).client;

        let rpc_system = RpcSystem::new(Box::new(network), Some(client));

        tokio::task::spawn_local(rpc_system);
    }
}
