use serde_json::json;
fn main() {
    let photo: Vec<_> = photocraft_engine::commands::command_specs().iter().map(|c| json!({"id":c.id,"label":c.label,"menu":c.menu,"shortcut":c.shortcut,"params":c.params,"journal":c.journal})).collect();
    let light: Vec<_> = lightcraft_engine::cmd::command_specs().iter().map(|c| json!({"id":c.id,"label":c.label,"menu":c.menu,"shortcut":c.shortcut,"params":c.params,"journal":c.journal})).collect();
    let film: Vec<_> = filmcraft_engine::commands::command_specs().iter().map(|c| json!({"id":c.id,"label":c.label,"menu":c.menu,"shortcut":c.shortcut,"params":c.params,"journal":c.journal})).collect();
    let vector:Vec<_>=vectorcraft_engine::command_specs().iter().map(|c|json!({"id":c.id,"label":c.label,"menu":c.menu,"shortcut":c.shortcut,"params":c.params,"journal":c.journal})).collect();
    let design:Vec<_>=designcraft_engine::command_specs().iter().map(|c|json!({"id":c.id,"label":c.label,"menu":c.menu,"shortcut":c.shortcut,"params":c.params,"journal":c.undoable})).collect();
    println!(
        "{}",
        json!({"photo":photo,"light":light,"film":film,"vector":vector,"design":design})
    );
}
