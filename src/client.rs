use capnp::capability::Promise;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem, pry};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::hello_world_capnp::hello_world;

// サーバー側で実装する HelloWorld のロジック
pub struct HelloWorldImpl;

pub async fn run_client(socket_path: &str) -> anyhow::Result<()> {
    // Unix ドメインソケットでサーバーに接続するが, 失敗した場合はエラーを出力する
    let stream = UnixStream::connect(socket_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to server: {}", e))?;

    // 読み取り / 書き込みのハーフに分ける（所有権の問題を回避するため）
    let (read_half, write_half) = stream.into_split();

    // Cap’n Proto RPC に渡すための互換ラッパーを適用
    let read_half = read_half.compat();
    let write_half = write_half.compat_write();

    // Cap’n Proto の VatNetwork（通信レイヤー）を構築
    let network = twoparty::VatNetwork::new(
        read_half,
        write_half,
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    );

    // Cap’n Proto の RPC システムを作成
    let mut rpc_system = RpcSystem::new(Box::new(network), None);

    // サーバーが持っている HelloWorld インターフェースを取得
    let hello_world: hello_world::Client =
        rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

    // RPC システムを非同期で動かす（spawn_local は LocalSet の中でしか使えない）
    tokio::task::spawn_local(rpc_system);

    // ここから RPC 呼び出し（sayHello）

    // リクエストを作成
    let mut request = hello_world.say_hello_request();
    {
        let mut req_struct = request.get().init_request();
        req_struct.set_name("Toru"); // name フィールド
        req_struct.set_value(123); // value フィールド
    }

    // サーバーに送信してレスポンスを待つ
    let response = request.send().promise.await?;

    // サーバーからの返答を取得
    let reply = response.get()?.get_reply()?;
    let message = reply.get_message()?.to_str()?; // Cap’n Proto Text を Rust の文字列に

    // サーバーからの返答を表示
    println!("[client] サーバーからの返答: {}", message);

    Ok(())
}