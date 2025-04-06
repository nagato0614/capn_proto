// server.rs をモジュールとして読み込む（サーバー機能）
mod client;
mod server;

// Cap’n Proto の自動生成された型と RPC クライアントのコード
#[path = "schema/hello_world_capnp.rs"]
mod hello_world_capnp;

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::net::UnixStream;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use client::run_client;
use hello_world_capnp::hello_world;
use server::run_server;

/// Server 側の HelloWorld のロジックを実装
async fn server(socket_path: &str) -> Result<(), capnp::Error> {
    // tokio の LocalSet（spawn_local を使うために必要）
    tokio::task::LocalSet::new()
        .run_until(run_server(socket_path))
        .await
        .expect("Failed to run server");
    Ok(())
}

/// Client 側の HelloWorld のロジックを実装
async fn client(socket_path: &str) -> Result<(), capnp::Error> {
    // クライアントモード開始（LocalSet の中で動かす）
    tokio::task::LocalSet::new()
        .run_until(run_client(socket_path))
        .await
        .expect("Failed to run client");
    Ok(())
}

/// Cap’n Proto RPC のサーバーとクライアントを実装
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let socket_path = "/tmp/capnp-demo.sock";

    // コマンドライン引数のリストを取得（最初の要素は実行ファイル名なのでスキップ）
    let args: Vec<String> = std::env::args().skip(1).collect();

    // 最初の引数で処理を分岐
    match args.first().map(|s| s.as_str()) {
        Some("--server") => {
            println!("[server] 起動中...");
            server(socket_path).await?;
        }
        Some("--client") => {
            println!("[client] 起動中...");
            client(socket_path).await?;
        }
        _ => {
            eprintln!(
                "Usage: {} --server | --client",
                std::env::args().next().unwrap()
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
