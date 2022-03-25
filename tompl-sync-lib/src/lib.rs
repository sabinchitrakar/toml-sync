use std::path::PathBuf;

use cargo_toml::Manifest;
use glob::GlobError;
use hyper::{
    body::{to_bytes, Buf},
    Client,
};
use tokio::io::{self, AsyncWriteExt as _};
pub enum SourceType {
    Remote,
    Local,
}
pub struct Source {
    pub path: String,
    pub source_type: SourceType,
}

pub struct SyncConfig {
    pub sources: Vec<Source>,
    pub destination: String,
}

pub struct TomlSync {
    config: SyncConfig,
}

impl TomlSync {
    pub fn new(config: SyncConfig) -> Self {
        TomlSync { config }
    }

    pub async fn load_source_tomls(&self) -> Vec<Manifest> {
        let bytes = self.load_source_bytes().await;
        return bytes
            .into_iter()
            .filter_map(|byte| Manifest::from_slice(&byte).ok())
            .collect::<Vec<Manifest>>();
    }

    pub async fn load_target_tomls(&self) -> Vec<Manifest> {
        let pattern = format!("{}/**/Cargo.toml", &self.config.destination);
        let globs = glob::glob(&pattern).expect("Invalid Glob expression");
        return globs
            .into_iter()
            .filter_map(|p| p.ok())
            .filter_map(|pb| std::fs::read(pb).ok())
            .filter_map(|bytes| Manifest::from_slice(&bytes).ok())
            .collect::<Vec<Manifest>>();
    }

    async fn load_source_bytes(&self) -> Vec<Vec<u8>> {
        let mut source_bytes = Vec::<Vec<u8>>::new();
        for source in &self.config.sources {
            match source.source_type {
                SourceType::Remote => {
                    if let Ok(uri) = source.path.parse() {
                        if let Ok(data) = Self::fetch_url(uri).await {
                            source_bytes.push(data);
                        } else {
                            println!("Failed to get data from {:?}", source.path)
                        }
                    } else {
                        print!("Invalid Url {:?}", &source.path)
                    }
                }
                SourceType::Local => {
                    if let Ok(res) = std::fs::read(source.path.clone()) {
                        source_bytes.push(res);
                    } else {
                        println!("Failed To read from {:?}", source.path)
                    }
                }
            }
        }
        return source_bytes;
    }

    async fn fetch_url(url: hyper::Uri) -> Result<Vec<u8>, hyper::Error> {
        let client = Client::new();

        let mut res = client.get(url).await?;

        println!("Response: {}", res.status());

        let bytes = to_bytes(res.body_mut()).await?;

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
