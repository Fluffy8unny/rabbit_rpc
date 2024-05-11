use serde::de::DeserializeOwned;
use serde::Serialize;


pub fn deserialize_vec_8<T:DeserializeOwned>(content:Vec<u8>)->T{
        let content_string = String::from_utf8(content).unwrap();
        serde_json::from_str(&content_string).expect("could not deserialize input")
}

pub fn serialize_vec_8<T:Serialize>(content:T)->Vec<u8>{
        serde_json::to_vec(&content).unwrap().to_owned()
}
