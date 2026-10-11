//! Small layered originals for browser integration checks, created by the real PhotoCraft engine.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = photocraft_engine::Session::default();
    session.execute(
        "file.new",
        serde_json::json!({"width":96,"height":64,"name":"Layers"}),
    )?;
    session.execute(
        "paint.bucket",
        serde_json::json!({"x":0,"y":0,"color":"#405060"}),
    )?;
    session.execute("shape.create", serde_json::json!({"kind":"rect","rect":[20,16,24,24],"fill":"#ff0000","name":"Editable square"}))?;
    let doc = &session.active().ok_or("Missing fixture")?.doc;
    std::fs::create_dir_all("test-results")?;
    std::fs::write(
        "test-results/layers.pcraft",
        photocraft_format::save_to_bytes(doc, &Default::default())?,
    )?;
    Ok(())
}
