// Copyright 2020-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::process::exit;
use std::sync::mpsc::channel;
use wry::WebViewBuilder;

fn main() -> wry::Result<()> {
  let html = r#"
    <!DOCTYPE html>
    <html>
      <head><meta charset="utf-8"></head>
      <body>
        <h1>Wry Headless Benchmark</h1>
        <script>
          document.addEventListener('DOMContentLoaded', () => {
            if (window.ipc) {
              window.ipc.postMessage('ready');
            }
          });
        </script>
      </body>
    </html>
  "#;

  let (tx, rx) = channel();

  let _webview = WebViewBuilder::new()
    .with_html(html)
    .with_ipc_handler(move |req| {
      if req.body() == "ready" {
        let _ = tx.send(());
      }
    })
    .build_headless()?;

  #[cfg(target_os = "linux")]
  {
    while rx.try_recv().is_err() {
      while gtk::events_pending() {
        gtk::main_iteration();
      }
      std::thread::sleep(std::time::Duration::from_millis(1));
    }
    exit(0);
  }

  #[cfg(not(target_os = "linux"))]
  {
    use tao::event_loop::{ControlFlow, EventLoop};
    let event_loop = EventLoop::new();
    event_loop.run(move |_, _, control_flow| {
      *control_flow = ControlFlow::Poll;
      if rx.try_recv().is_ok() {
        exit(0);
      }
    });
  }

  #[allow(unreachable_code)]
  Ok(())
}
