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
/// * `event_listener` - クライアントから登録されたイベントリスナーを保持する。
pub struct HelloWorldImpl {
    event_listener: Arc<Mutex<Option<hello_world::event_listener::Client>>>,
}

/// HelloWorld RPC サーバーのメソッド実装
impl hello_world::Server for HelloWorldImpl {
    /// クライアントからの `sayHello` RPC リクエストを処理します。
    ///
    /// # 引数
    /// * `params` - クライアントからのリクエストパラメータ（HelloRequest 構造体）。
    /// * `results` - クライアントへ返すレスポンス（HelloReply 構造体）。
    ///
    /// # 戻り値
    /// * 成功時は `Promise::ok(())` を返します。
    fn say_hello(
        &mut self,
        params: hello_world::SayHelloParams,
        mut results: hello_world::SayHelloResults,
    ) -> Promise<(), capnp::Error> {
        // リクエストの取得とパース
        let request = pry!(pry!(params.get()).get_request());
        let name = pry!(request.get_name());
        let value = request.get_value();

        println!("[server] name='{:?}', value={}", name, value);

        // レスポンスメッセージを作成してクライアントに返す
        let mut reply = results.get().init_reply();
        let message = format!("Hello {:?}, I got {}", name, value);
        reply.set_message(&message);

        Promise::ok(())
    }

    /// クライアントからの `subscribeEvents` RPC リクエストを処理します。
    ///
    /// # 引数
    /// * `params` - クライアントから渡されたイベントリスナー。
    /// * `_results` - 戻り値は今回は使用しません。
    ///
    /// # 戻り値
    /// * 成功時は `Promise::ok(())` を返します。
    fn subscribe_events(
        &mut self,
        params: hello_world::SubscribeEventsParams,
        _results: hello_world::SubscribeEventsResults,
    ) -> Promise<(), capnp::Error> {
        // クライアントから送られてきた listener を保持
        let listener = pry!(pry!(params.get()).get_listener());

        {
            let mut guard = self.event_listener.lock().unwrap();
            *guard = Some(listener.clone());
        }

        println!("[server] イベントリスナー登録完了！");

        let listener_ref = Arc::clone(&self.event_listener);

        // 非同期で通知を送るタスクを起動
        tokio::task::spawn_local(async move {
            let mut failure_count = 0;

            loop {
                sleep(Duration::from_secs(1)).await;

                let maybe_listener = {
                    let guard = listener_ref.lock().unwrap();
                    guard.clone()
                };

                if let Some(listener) = maybe_listener {
                    // クライアントにイベントを送信するリクエストを作成
                    let mut request = listener.on_event_request();
                    request.get().set_message("サーバーからの定期通知です！");

                    match request.send().promise.await {
                        Ok(_) => {
                            println!("[server] イベント送信成功！");
                            failure_count = 0; // 成功したら失敗カウントリセット
                        }
                        Err(e) => {
                            eprintln!("[server] イベント送信失敗: {:?}", e);
                            failure_count += 1;

                            if failure_count >= 5 {
                                println!("[server] 連続で 5 回失敗したため、通知を停止します。");
                                break; // 5 回失敗したら通知停止
                            }
                        }
                    }
                } else {
                    println!("[server] リスナーが存在しないため通知をスキップします。");
                }
            }
        });

        Promise::ok(())
    }
}

/// Cap’n Proto RPC サーバーを起動します。
///
/// # 引数
/// * `socket_path` - 通信に使用する Unix ドメインソケットのパス。
///
/// # 戻り値
/// * `Result<(), anyhow::Error>` - 起動成功またはエラー。
pub async fn run_server(socket_path: &str) -> anyhow::Result<()> {
    // 既存のソケットファイルがあれば削除
    let _ = std::fs::remove_file(socket_path);

    // Unix ドメインソケットで待ち受け開始
    let listener = UnixListener::bind(socket_path)?;

    // イベントリスナーを共有メモリとして保持
    let event_listener = Arc::new(Mutex::new(None));

    loop {
        // クライアントからの接続を受け付ける
        let (stream, _) = listener.accept().await?;
        let (read_half, write_half) = stream.into_split();
        let read_half = read_half.compat();
        let write_half = write_half.compat_write();

        // Cap’n Proto の通信ネットワークを構築
        let network = twoparty::VatNetwork::new(
            read_half,
            write_half,
            rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );

        // サービス実装を作成し、クライアントからのリクエストに備える
        let service_impl = HelloWorldImpl {
            event_listener: Arc::clone(&event_listener),
        };

        // Cap’n Proto RPC システムを起動
        let client = capnp_rpc::new_client::<hello_world::Client, _>(service_impl).client;

        let rpc_system = RpcSystem::new(Box::new(network), Some(client));

        // 非同期タスクとして RPC システムを実行
        tokio::task::spawn_local(rpc_system);
    }
}
