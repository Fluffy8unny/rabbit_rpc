# rabbit_rpc
RPC functionality for Rust's amqprs library

## Motivation
The offical [RabbitMQ webiste](https://www.rabbitmq.com/tutorials) currently has no Rust example for RPC functionality. Furthermore I wasn't able to find a similar crate. RabbitMQ is very powerful, but not easy to use as a beginner. I hope this repository will be helpful for someone needing RPC functionality.

## Example
In this example we reproduce the fibonnachi example from the offical rabbimq website

### Server
You simply pass a callback function to the Server constructor. All arguments and the return type of the function have to fulfill the serde::Serializa and Deserialize traits(we could have just used fn(u32)->u32 in this example though. There is no need to define your own types).

```rust
use amqprs::connection::OpenConnectionArguments;
use serde::{Deserialize, Serialize};

use rabbit_rpc::start_rpc_server;

//exmaple payload
#[derive(Debug, Serialize, Deserialize)]
struct TestMsg {
    number : u32,
    }

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
let client_args  = OpenConnectionArguments::new("localhost", 5672, "guest", "guest");
let server_args  = client_args.clone();

let rpc_fn = Box::new(move |msg: TestMsg| {
                fn fibbonacchi( n : u32)->u32 { match n {
                    0     => panic!("0 is no valid input for fibonacchi")
                    1 | 2 => 1,
                    _     => fibbonacchi(n-1) + fibbonacchi(n-2),
                } }
                fibbonacchi(msg.number)
            });
start_rpc_server( server_args, //connection arguments e.g. ip and port of the RabbitMQ server 
                  "test",      // name of the rpc function. You use this in the client to call it
                  rpc_fn,      //the callback, that will be called for each msg
                  true         //should auto ack be used or do we need to manually ack the msg. You probably can leave it as true
                ).await.unwrap()
}
```

### Client
First instantiate a client, then use it to perform the rpc call
```rust
let mut client = RPCClient::new(&client_args).await.unwrap();
let out: u32 = client
    .call(
        "test",         //name of the RPC server
        TestMsg {       //payload
            number: 5,
        },
    )
    .await
    .unwrap();
```

### Todos
Adding error handling would be nice e.g. catching errors in the server, serializing it in any way shape or form and telling the client that somethign went wrong.
