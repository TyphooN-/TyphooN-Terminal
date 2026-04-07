#![allow(clippy::needless_return)]

mod app;

use wasm_bindgen::prelude::*;

/// Entry point called from index.html via trunk.
#[wasm_bindgen(start)]
pub fn main() {
    // Simple console logging for WASM
    console_error_panic_hook::set_once();

    let options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let window = web_sys::window().expect("no window");
        let document = window.document().expect("no document");
        let canvas = document
            .get_element_by_id("typhoon_canvas")
            .expect("no canvas element")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("not a canvas");

        let runner = eframe::WebRunner::new();
        runner
            .start(
                canvas,
                options,
                Box::new(|cc| Ok(Box::new(app::WebApp::new(cc)))),
            )
            .await
            .expect("Failed to start eframe");
    });
}
