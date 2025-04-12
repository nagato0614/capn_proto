use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::net::UnixListener;
use tokio::time::{sleep, Duration};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use std::sync::{Arc, Mutex};

use crate::hello_world_capnp::hello_world;

/// サーバー側で実装する HelloWorld のロジック
///
/// # フィールド
/// * `event_listeners` - クライアントから登録されたイベントリスナーを保持する。
pub struct HelloWorldImpl {
    event_listeners: Arc<Mutex<Vec<hello_world::event_listener::Client>>>,
}

/// HelloWorld RPC サーバーのメソッド実装
impl hello_world::Server for HelloWorldImpl {
    /// クライアントからの `sayHello` RPC リクエストを処理します。
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

    /// クライアントからの `subscribeEvents` RPC リクエストを処理します。
    fn subscribe_events(
        &mut self,
        params: hello_world::SubscribeEventsParams,
        _results: hello_world::SubscribeEventsResults,
    ) -> Promise<(), capnp::Error> {
        let listener = pry!(pry!(params.get()).get_listener());

        {
            let mut guard = self.event_listeners.lock().unwrap();
            guard.push(listener.clone());
        }

        println!("[server] 新しいイベントリスナー登録完了！");

        Promise::ok(())
    }
}

/// Cap’n Proto RPC サーバーを起動します。
///
/// # 引数
/// * `socket_path` - 通信に使用する Unix ドメインソケットのパス。
pub async fn run_server(socket_path: &str) -> anyhow::Result<()> {
    // 既存のソケットファイルがあれば削除
    let _ = std::fs::remove_file(socket_path);

    // Unix ドメインソケットで待ち受け開始
    let listener = UnixListener::bind(socket_path)?;

    // イベントリスナーを共有メモリとして保持
    let event_listeners = Arc::new(Mutex::new(Vec::<hello_world::event_listener::Client>::new()));

    // 通知を送るループをサーバー起動時に一度だけ起動
    {
        let listener_ref = Arc::clone(&event_listeners);

        tokio::task::spawn_local(async move {
            loop {
                sleep(Duration::from_secs(1)).await;

                let mut to_remove = Vec::new();

                {
                    let listeners = listener_ref.lock().unwrap();
                    for (index, listener) in listeners.iter().enumerate() {
                        let mut request = listener.on_event_request();
                        request.get().set_message("サーバーからの定期通知です！");

                        match request.send().promise.await {
                            Ok(_) => {
                                println!("[server] イベント送信成功！ index={}", index);
                            }
                            Err(e) => {
                                eprintln!("[server] イベント送信失敗: {:?} index={}", e, index);
                                to_remove.push(index);
                            }
                        }
                    }
                }

                // 失敗したリスナーを削除
                if !to_remove.is_empty() {
                    let mut listeners = listener_ref.lock().unwrap();
                    for &index in to_remove.iter().rev() {
                        listeners.remove(index);
                        println!("[server] イベントリスナー削除 index={}", index);
                    }
                }
            }
        });
    }

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

        let service_impl = HelloWorldImpl {
            event_listeners: Arc::clone(&event_listeners),
        };

        let client = capnp_rpc::new_client::<hello_world::Client, _>(service_impl).client;

        let rpc_system = RpcSystem::new(Box::new(network), Some(client));

        tokio::task::spawn_local(rpc_system);
    }
}
