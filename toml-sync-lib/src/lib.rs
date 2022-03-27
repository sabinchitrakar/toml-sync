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
    pub target_manifests:HashMap<String,Manifest>,
    pub source_manifests:HashMap<String,Manifest>,
    pub source_versions: HashMap<String, Vec<VersionInfo>>,
    pub target_versions: HashMap<String, Vec<VersionInfo>>,
}
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub path: String,
    pub version: String,
}

pub struct Match {
    pub dependency:String,
    pub targets:Vec<VersionInfo>,
    pub sources:Vec<VersionInfo>,
    pub are_equal:bool,
}



impl Display for VersionInfo {
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
            source_manifests:HashMap::new(),
            target_manifests:HashMap::new()
        }
    }

    pub async fn scan(&mut self) {
        let source_manifests = self.load_source_tomls().await;
        let target_manifests = self.load_tomls(self.config.destination.clone()).await;
        for manifest in source_manifests {
            Self::extract_dependencies(&mut (*self).source_versions, manifest.0.clone(), &manifest.1);
            self.source_manifests.insert(manifest.0, manifest.1);
        }

        for manifest in target_manifests {
            Self::extract_dependencies(&mut (*self).target_versions, manifest.0.clone(), &manifest.1);
            self.target_manifests.insert(manifest.0, manifest.1);
        }
    }

    pub fn show_diff(&self) {
        let mut table = Table::new();
        table.add_row(row!["dependency", "target_versions", "source_versions"]);

        let intersects =self.intersects();

        for intersect in intersects {
           
            let key = intersect.dependency.clone();
            
            let source_version_str = intersect.sources
                .into_iter()
                .map(|t|Self::print_version_info(&t))
                .collect::<Vec<String>>()
                .join("\n");
            let target_versions_str = intersect.targets
                .into_iter()
                .map(|t|Self::print_version_info(&t))
                .collect::<Vec<String>>()
                .join("\n");

                
            if intersect.are_equal{
                table.add_row(row![FG=>key, target_versions_str, source_version_str]);
            }else{
                table.add_row(row![FR=>key, target_versions_str, source_version_str]);
            }
            
            
        }
        table.printstd();
    }

    fn intersects(&self)-> Vec<Match> {
       
        let mut matches:Vec<Match>=vec![];
        for (key, _value) in &self.target_versions {
            if self.source_versions.contains_key(key) {
                let mut intersect=Match{
                    dependency:key.clone(),
                    targets:self.target_versions.get(key).map(|v| v.to_vec()).unwrap(),
                    sources: self.source_versions.get(key).map(|v| v.to_vec()).unwrap(),
                    are_equal:false
                };
                let target_set = Self::get_versions(&intersect.targets);
                let source_set = Self::get_versions(&intersect.sources);
                let are_equal =target_set.difference(&source_set).count()==0;
                intersect.are_equal=are_equal;

                matches.push(intersect);
                
            }
        }
        return matches;
    }

    fn print_version_info(target_info: &VersionInfo) -> String {
        format!(
            "path:{} \nversion: {}",
            target_info.path,
            target_info.version.clone()
        )
    }

    fn get_versions(info:&Vec<VersionInfo>)->HashSet<String>{
        return info.into_iter()
        .map(|t|{t.version.to_string()})
        .collect::<HashSet<String>>();
    }

    pub fn extract_dependencies(
        map: &mut HashMap<String, Vec<VersionInfo>>,
        path: String,
        manifest: &Manifest,
    ) {
        for dependency in &manifest.dependencies {
            let dependency_key = dependency.0;
            let version = dependency.1.detail().and_then(|d| d.version.clone());
            println!("dependency:{:?} version:{:?}", dependency_key, version);
            let target_info = VersionInfo {
                version:version.unwrap_or_default(),
                path: path.clone(),
            };
            map.entry(dependency_key.clone())
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
