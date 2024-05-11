
use amqprs::{
    channel::{
        BasicAckArguments, BasicConsumeArguments, BasicPublishArguments, BasicRejectArguments,
        Channel, ConsumerMessage, QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    BasicProperties
};

use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedReceiver;
use uuid::Uuid;

use crate::util::{serialize_vec_8,deserialize_vec_8};

pub struct RPCClient {
    channel     : Channel,
    _connection : Connection,
    queue       : String,
    msg_rx      : UnboundedReceiver<ConsumerMessage>,
}

impl RPCClient {
    pub async fn new(
        connection_arguments: &OpenConnectionArguments,
    ) -> Result<RPCClient, Box<dyn std::error::Error>> {
        let _connection   = Connection::open(connection_arguments).await?;
        let channel       = _connection.open_channel(None).await?;
        let queue_args    = QueueDeclareArguments::default().exclusive(true).finish();
        let (queue, _, _) = channel.queue_declare(queue_args).await?.unwrap();

        let consume_args = BasicConsumeArguments::new(queue.as_str(), &format!("rpc_{queue}"))
            .manual_ack(true)
            .finish();
        let (_, msg_rx) = channel.basic_consume_rx(consume_args).await?;
        Ok(RPCClient {
            channel,
            _connection,
            queue,
            msg_rx,
        })
    }

    pub async fn call<T: 'static + Serialize + Send, U: DeserializeOwned>(
        &mut self,
        rpc_name   : &str,
        input_data : T,
    ) -> Result<U, Box<dyn std::error::Error>> {
        //generate correlation id, if multiple calls are done at the same time
        //this allows us to identify who called
        let correlation_id = Uuid::new_v4().to_string();

        let properties = BasicProperties::default()
            .with_correlation_id(&correlation_id)
            .with_reply_to(self.queue.as_str())
            .finish();

        //query server
        self.channel
            .basic_publish(
                properties,
                serialize_vec_8(input_data),
                BasicPublishArguments::new("", rpc_name),
            )
            .await?;

        //wait for reply
        while let Some(ConsumerMessage {
            deliver,
            basic_properties,
            content,
            ..
        }) = self.msg_rx.recv().await
        {
            if let Some(_basic_properties) = basic_properties {
                if _basic_properties.correlation_id().unwrap() == &correlation_id {

                    //this msg was for us, we need to ack it       
                    let args = BasicAckArguments::new(deliver.unwrap().delivery_tag(), false);
                    self.channel.basic_ack(args).await.unwrap();
                    
                    //deserialize content and return result
                    let content_u8 =content.clone().unwrap();
                    return Ok(deserialize_vec_8(content_u8));
                }
            }

            //this msg wasn't for us
            self.channel
                .basic_reject(BasicRejectArguments::default())
                .await
                .unwrap();
        }
        Err("could not obtain RPC result".into())
    }
}
