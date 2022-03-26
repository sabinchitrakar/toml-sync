use dialoguer::{Input, Select, console::Term, theme::ColorfulTheme};
use toml_sync_lib::{Source, SourceType, TomlSync, SyncConfig};

#[tokio::main]
async fn main() {
    let sources : String = Input::new()
    .with_prompt("Add Sources Comma Separated :")
    .interact_text().unwrap();

    let target=Input::<String>::new()
    .with_prompt("Directory To Search :")
    .interact_text().unwrap();

    let sources_parsed= sources.split(",").map(|t|{
        let source_type = if t.starts_with("http") { SourceType::Remote } else { SourceType::Local};
        let source = Source{
            path:t.to_string(),
            source_type
        };
        source
    }).collect::<Vec<Source>>();

    let mut sync = TomlSync::new(SyncConfig{
        destination:target,
        sources:sources_parsed
    });
    sync.scan().await;

    show_menu(sync);

}


pub fn show_menu(sync:TomlSync){

    let items = vec!["Show Diff", "Sync Versions"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .interact_on_opt(&Term::stderr()).unwrap();

    match selection {
        Some(index) => {
            match index {
                0 =>{
                  sync.show_diff();
                },
                _ =>{}
            }
        },
        None => println!("No Action Selected")
    }

}
