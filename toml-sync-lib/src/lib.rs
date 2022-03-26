#[macro_use]
extern crate prettytable;
use hyper_tls::HttpsConnector;
use std::{collections::{HashMap, HashSet}, fmt::Display};

use cargo_toml::Manifest;
use hyper::{body::to_bytes, Client};

use prettytable::Table;
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
    pub source_versions: HashMap<String, Vec<TargetInfo>>,
    pub target_versions: HashMap<String, Vec<TargetInfo>>,
}
#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub path: String,
    pub version: String,
}



impl Display for TargetInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "path: {} \n versions: {:?}", self.path, self.version)
    }
}



impl TomlSync {
    pub fn new(config: SyncConfig) -> Self {
        TomlSync {
            config,
            source_versions: HashMap::new(),
            target_versions: HashMap::new(),
        }
    }

    pub async fn scan(&mut self) {
        let source_manifests = self.load_source_tomls().await;
        let target_manifests = self.load_tomls(self.config.destination.clone()).await;
        for manifest in source_manifests {
            Self::extract_dependencies(&mut (*self).source_versions, manifest.0, manifest.1);
        }

        for manifest in target_manifests {
            Self::extract_dependencies(&mut (*self).target_versions, manifest.0, manifest.1);
        }
    }

    pub fn show_diff(&self) {
        let mut table = Table::new();
        table.add_row(row!["dependency", "target_versions", "source_versions"]);

        let intersects =self.intersects();

        for (key,value) in intersects {
            let target_set = Self::get_versions(&value.1);
            let source_set = Self::get_versions(&value.0);
            let are_equal =target_set.difference(&source_set).count()==0;
            
            
            let source_version_str = value.0
                .into_iter()
                .map(|t|Self::print_target_info(&t))
                .collect::<Vec<String>>()
                .join("\n");
            let target_versions_str = value.1
                .into_iter()
                .map(|t|Self::print_target_info(&t))
                .collect::<Vec<String>>()
                .join("\n");
            if are_equal{
                table.add_row(row![FG=>key, target_versions_str, source_version_str]);
            }else{
                table.add_row(row![FR=>key, target_versions_str, source_version_str]);
            }
            
            
        }
        table.printstd();
    }

    fn intersects(&self)-> HashMap<String,(Vec<TargetInfo>,Vec<TargetInfo>)> {
        let mut intersects = HashMap::<String, (Vec<TargetInfo>, Vec<TargetInfo>)>::new();
        for (key, _value) in &self.target_versions {
            if self.source_versions.contains_key(key) {
                intersects.insert(
                    key.to_string(),
                    (
                        self.source_versions.get(key).map(|v| v.to_vec()).unwrap(),
                        self.target_versions.get(key).map(|v| v.to_vec()).unwrap(),
                    ),
                );
            }
        }
        return intersects;
    }

    fn print_target_info(target_info: &TargetInfo) -> String {
        format!(
            "path:{} \nversion: {}",
            target_info.path,
            target_info.version.clone()
        )
    }

    fn get_versions(info:&Vec<TargetInfo>)->HashSet<String>{
        return info.into_iter()
        .map(|t|{t.version.to_string()})
        .collect::<HashSet<String>>();
    }

    pub fn extract_dependencies(
        map: &mut HashMap<String, Vec<TargetInfo>>,
        path: String,
        manifest: Manifest,
    ) {
        for dependency in manifest.dependencies {
            let dependency_key = dependency.0;
            let version = dependency.1.detail().and_then(|d| d.version.clone());
            println!("dependency:{:?} version:{:?}", dependency_key, version);
            let target_info = TargetInfo {
                version:version.unwrap_or_default(),
                path: path.clone(),
            };
            map.entry(dependency_key)
                .or_insert(vec![])
                .push(target_info);
        }
    }

    pub async fn load_tomls(&self, directory: String) -> Vec<(String, Manifest)> {
        let pattern = format!("{}/**/Cargo.toml", directory);
        let globs = glob::glob(&pattern).expect("Invalid Glob expression");
        return globs
            .into_iter()
            .filter_map(|p| p.ok())
            .filter_map(|pb| std::fs::read(pb.clone()).map(|bytes| (bytes, pb)).ok())
            .filter_map(|(bytes, pb)| {
                let result = Manifest::from_slice(&bytes);
                match result {
                    Ok(manifest) => Some((pb.clone().to_str().unwrap_or("").to_string(), manifest)),
                    Err(err) => {
                        println!("{:?}", err);
                        None
                    }
                }
            })
            .collect::<Vec<(String, Manifest)>>();
    }

    async fn load_source_tomls(&self) -> Vec<(String, Manifest)> {
        let mut source_bytes = Vec::<(String, Manifest)>::new();
        for source in &self.config.sources {
            match source.source_type {
                SourceType::Remote => {
                    if let Ok(uri) = source.path.parse() {
                        if let Ok(data) = Self::fetch_url(uri).await {
                            // source_bytes.push((source.path.clone(),data));
                            let result = Manifest::from_slice(&data);
                            match result {
                                Ok(manifest) => source_bytes.push((source.path.clone(), manifest)),
                                Err(err) => {
                                    println!("Failed to Parse {:?}", err);
                                }
                            }
                        } else {
                            println!("Failed to get data from {:?}", source.path)
                        }
                    } else {
                        print!("Invalid Url {:?}", &source.path)
                    }
                }
                SourceType::Local => {
                    let mut res = self.load_tomls(source.path.clone()).await;
                    source_bytes.append(&mut res);
                }
            }
        }
        return source_bytes;
    }

    async fn fetch_url(url: hyper::Uri) -> Result<Vec<u8>, hyper::Error> {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        let mut res = client.get(url).await?;

        println!("Response: {}", res.status());

        let bytes = to_bytes(res.body_mut()).await?;

        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Source, SourceType, SyncConfig, TomlSync};

    #[tokio::test]
    async fn it_works() {
        let config = SyncConfig{
           sources:vec![Source{
               path:"https://raw.githubusercontent.com/paritytech/frontier/master/frame/evm/Cargo.toml".to_owned(),
               source_type:SourceType::Remote
           }],
           destination:"./../".to_owned(),
       };

        let mut toml_sync = TomlSync::new(config);
        toml_sync.scan().await;
        toml_sync.show_diff();
    }
}