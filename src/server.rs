use amqprs::{
    channel::{
        BasicAckArguments, BasicConsumeArguments, BasicPublishArguments, BasicRejectArguments,
        Channel, QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    consumer::AsyncConsumer,
    BasicProperties, Deliver,
};

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::util::{serialize_vec_8,deserialize_vec_8};

struct RPCConsumer<T, U> {
    callback: Box<dyn Fn(U) -> T + Send>,
    auto_ack: bool,
}

#[async_trait]
impl<T: 'static + Serialize + Send, U: DeserializeOwned> AsyncConsumer for RPCConsumer<T, U> {
    async fn consume(
        &mut self,
        channel          : &Channel,
        deliver          : Deliver,
        _basic_properties: BasicProperties,
        content          : Vec<u8>,
    ) {
        //get reply x, call f(x)
        let content_t = deserialize_vec_8(content); 
        let reply = (&self.callback)(content_t);
       
        //return f(x)
        let corr_id = _basic_properties
            .correlation_id()
            .unwrap_or(&"".to_string())
            .clone();

        let reply_queue = match _basic_properties.reply_to() {
            Some(x) => x,
            None => {
                println!("message had no reply to rejecting.");
                channel
                    .basic_reject(BasicRejectArguments::default())
                    .await
                    .unwrap();
                return;
            }
        };

        let properties = BasicProperties::default()
            .with_correlation_id(&corr_id)
            .finish();
        channel
            .basic_publish(
                properties,
                serialize_vec_8(reply),
                BasicPublishArguments::new("", reply_queue),
            )
            .await
            .unwrap();

        //finally handle acking
        if !self.auto_ack {
            let args = BasicAckArguments::new(deliver.delivery_tag(), false);
            channel.basic_ack(args).await.unwrap();
        }
    }
}

pub async fn start_rpc_server<T: 'static + Serialize + Send, U: DeserializeOwned + 'static>(
    connection_arguments : OpenConnectionArguments,
    rpc_name             : &str,
    callback             : Box<dyn Fn(U) -> T + Send>,
    auto_ack             : bool,
) -> Result<(), Box<dyn std::error::Error>> {
    //setup connection
    let connection = Connection::open(&connection_arguments).await?;
    let channel    = connection.open_channel(None).await?;
    channel
        .queue_declare(QueueDeclareArguments::durable_client_named(rpc_name))
        .await?
        .unwrap();

    //setup consumer
    let args = BasicConsumeArguments::new(rpc_name, &format!("rpc {rpc_name}"))
                                         .auto_ack(auto_ack)
                                         .finish();
    channel
        .basic_consume(
            RPCConsumer {
                callback,
                auto_ack: args.no_ack,
            },
            args,
        )
        .await
        .unwrap();

    //consume forver
    let guard = tokio::sync::Notify::new();
    guard.notified().await;
    Ok(())
}
