use std::{path::PathBuf, collections::HashMap};

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
    pub config: SyncConfig,
    pub source_versions:HashMap<String,Vec<String>>,
    pub target_versions:HashMap<String,TargetInfo>
}

pub struct TargetInfo {
    pub path:PathBuf,
    pub version:String,
}

impl TomlSync {

    pub fn new(config: SyncConfig) -> Self {
        TomlSync { 
            config ,
            source_versions:HashMap::new(),
            target_versions:HashMap::new(),
        }
    }

    pub async fn sync(&self){
        let source_manifest=self.load_source_tomls().await;
        let target_manifests =self.load_target_tomls().await;
        for manifest in source_manifest {
            println!("{:#?}",manifest);
        }

        for manifest in target_manifests {
            println!("{:#?}",manifest);
        }
    }

    pub async fn load_source_tomls(&self) -> Vec<Manifest> {
        let bytes = self.load_source_bytes().await;
        return bytes
            .into_iter()
            .filter_map(|byte| Manifest::from_slice(&byte).ok())
            .collect::<Vec<Manifest>>();
    }

    pub async fn load_target_tomls(&self) -> Vec<(Manifest,PathBuf)> {
        let pattern = format!("{}/**/Cargo.toml", &self.config.destination);
        let globs = glob::glob(&pattern).expect("Invalid Glob expression");
        return globs
            .into_iter()
            .filter_map(|p| p.ok())
            .filter_map(|pb| std::fs::read(pb.clone()).map(|bytes|{(bytes,pb)}).ok())
            .filter_map(|(bytes,pb)| Manifest::from_slice(&bytes).map(|m|{(m,pb)}).ok())
            .collect::<Vec<(Manifest,PathBuf)>>();
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
    use crate::{SyncConfig, SourceType, Source, TomlSync};

    #[tokio::test]
    async fn it_works() {
       let config = SyncConfig{
           sources:vec![Source{
               path:"./../../toml-sync/Cargo.toml".to_owned(),
               source_type:SourceType::Local
           }],
           destination:"./".to_owned(),
       };

       let toml_sync= TomlSync::new(config);
       toml_sync.sync().await;


    }
}
