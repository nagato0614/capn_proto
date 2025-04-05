@0x9c50f680ea4721cc;

interface HelloWorld {
    struct HelloRequest {
        name @0 :Text;
        value @1 :Int32;
    }

    struct HelloReply {
        message @0 :Text;
    }

    # クライアント → サーバー : sayHello RPC
    sayHello @0 (request: HelloRequest) -> (reply: HelloReply);

    # サーバーからクライアントへのコールバック用インターフェース
    interface EventListener {
        onEvent @0 (message :Text) -> ();
    }

    # クライアント → サーバー: イベントリスナー登録
    subscribeEvents @1 (listener :EventListener) -> ();
}
