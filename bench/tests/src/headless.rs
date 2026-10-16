// Copyright 2020-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::process::exit;
use wry::http::Request;
use wry::WebViewBuilder;

fn main() -> wry::Result<()> {
  #[cfg(not(target_os = "linux"))]
  let event_loop = tao::event_loop::EventLoop::new();

  let html = r#"
    <!DOCTYPE html>
    <body>
    <script>
    document.addEventListener('DOMContentLoaded', () => {
      ipc.postMessage('dom-loaded')
    })
    </script>
    </body>
  "#;

  let handler = |req: Request<String>| {
    if req.body() == "dom-loaded" {
      exit(0);
    }
  };

  let _webview = WebViewBuilder::new()
    .with_html(html)
    .with_ipc_handler(handler)
    .build_headless()?;

  #[cfg(target_os = "linux")]
  {
    loop {
      while gtk::events_pending() {
        gtk::main_iteration();
      }
      std::thread::sleep(std::time::Duration::from_millis(1));
    }
  }

  #[cfg(not(target_os = "linux"))]
  {
    use tao::event_loop::ControlFlow;
    event_loop.run(move |_, _, control_flow| {
      *control_flow = ControlFlow::Wait;
    });
  }

  #[allow(unreachable_code)]
  Ok(())
}
