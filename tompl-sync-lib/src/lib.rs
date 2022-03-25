use hyper::{Client, body::{Buf, to_bytes}};
use tokio::io::{self, AsyncWriteExt as _};
pub enum SourceType {
    Remote,
    Local
}
pub struct Source {
    pub path:String,
    pub source_type: SourceType
}

pub struct SyncConfig {
    pub sources:Vec<Source>,
    pub destination:String,
}

pub struct TomlSync {
    config:SyncConfig
}

impl TomlSync {
    pub fn new(config:SyncConfig)->Self{
        TomlSync{
            config
        }
    }

    pub fn load_sources(&self) {

    }

    async fn fetch_url(url: hyper::Uri) ->Result<Vec<u8>,hyper::Error>{
        let client = Client::new();
    
        let mut res = client.get(url).await?;
    
        println!("Response: {}", res.status());

        let bytes =to_bytes(res.body_mut()).await?;

        Ok(bytes.to_vec())
        
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
