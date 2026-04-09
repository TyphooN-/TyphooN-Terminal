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
        // Each of these "cannot happen" in a normal browser context, but we log
        // to the console instead of panicking so the user gets a visible error
        // in the dev tools rather than a cryptic wasm trap. Per ADR-082.
        let Some(window) = web_sys::window() else {
            web_sys::console::error_1(&"TyphooN: no window object — not running in a browser?".into());
            return;
        };
        let Some(document) = window.document() else {
            web_sys::console::error_1(&"TyphooN: no document object".into());
            return;
        };
        let Some(canvas_elem) = document.get_element_by_id("typhoon_canvas") else {
            web_sys::console::error_1(&"TyphooN: #typhoon_canvas element missing from index.html".into());
            return;
        };
        let canvas = match canvas_elem.dyn_into::<web_sys::HtmlCanvasElement>() {
            Ok(c) => c,
            Err(_) => {
                web_sys::console::error_1(&"TyphooN: #typhoon_canvas is not a <canvas> element".into());
                return;
            }
        };

        let runner = eframe::WebRunner::new();
        if let Err(e) = runner
            .start(
                canvas,
                options,
                Box::new(|cc| Ok(Box::new(app::WebApp::new(cc)))),
            )
            .await
        {
            web_sys::console::error_1(&format!("TyphooN: eframe start failed: {e:?}").into());
        }
    });
}
