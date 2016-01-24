extern crate pixelport_app;
extern crate pixelport_viewport;
extern crate pixelport_document;
extern crate image;

use pixelport_app::*;
use pixelport_document::*;

use std::path::{Path, PathBuf};
use std::process::Command;
use pixelport_viewport::resources::*;
use std::fs;

fn headless_document_opts(filename: &str) -> AppOptions {
    let path = Path::new(&filename);
    let root_path = path.parent().unwrap().to_path_buf();
    AppOptions {
        viewport: pixelport_viewport::ViewportSubSystemOptions {
            fullscreen: false,
            multisampling: 0,
            vsync: false,
            headless: true,
            window_size: Some((100, 100))
        },
        port: 0,
        document: Document::from_file(path).unwrap(),
        root_path: root_path,
        time_progression: TimeProgression::Fixed { step_ms: 16 },
        min_frame_ms: None
    }
}

fn setup_app(name: &str) -> App {
	let mut app = App::new(headless_document_opts(&format!("../examples/{}.pml", name)));
	app.update();
    app.viewport.await_resources();
    app.update();
    app
}
fn compare_screenshot(name: &str, app: &App) {
	let found = app.viewport.screenshot().unwrap().to_rgba();
    fs::create_dir_all("tests/found");
    found.save(&Path::new(&format!("tests/found/{}.png", name)));
    let expected = TextureSource::from_file(&Path::new(&format!("tests/expected/{}.png", name))).unwrap().to_rgba();
    assert!(found.diff(&expected) < 0.01);
}
fn test_example(name: &str) {
    let app = setup_app(name);
    compare_screenshot(name, &app);
}

#[test]
fn test_examples_box() {
    test_example("box");
}

#[test]
fn test_examples_text() {
    test_example("text");
}

#[test]
fn test_examples_render_to_texture() {
    test_example("render_to_texture");
}

#[test]
fn test_examples_sample_frame_buffer() {
    test_example("sample_frame_buffer");
}

#[test]
fn test_visualize_entity_renderer_bounding() {
    let mut app = setup_app("box");
    app.viewport.visualize_entity_bounding = Some(3);
    app.update();
    compare_screenshot("viz_bounding", &app);
}
