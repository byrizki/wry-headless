// Copyright 2020-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::process::exit;
use std::sync::mpsc::channel;
use wry::{PageLoadEvent, PrintToPdfOptions, WebViewBuilder};

const HTML_CONTENT: &str = r#"
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Benchmark PDF Document</title>
  <style>
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      margin: 40px;
      color: #333;
      line-height: 1.6;
    }
    h1 {
      color: #2c3e50;
      border-bottom: 2px solid #3498db;
      padding-bottom: 10px;
    }
    .header-table {
      width: 100%;
      border-collapse: collapse;
      margin-bottom: 30px;
    }
    .header-table th, .header-table td {
      border: 1px solid #ddd;
      padding: 12px;
      text-align: left;
    }
    .header-table th {
      background-color: #f8f9fa;
      font-weight: 600;
    }
    .invoice-total {
      text-align: right;
      font-size: 1.2em;
      font-weight: bold;
      margin-top: 20px;
    }
  </style>
</head>
<body>
  <h1>Performance &amp; Quality Benchmark Report</h1>
  <p>Generated via cross-platform native headless webview PDF pipeline.</p>
  <table class="header-table">
    <thead>
      <tr>
        <th>Item</th>
        <th>Description</th>
        <th>Status</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>Headless Core</td>
        <td>Windowless off-screen rendering engine</td>
        <td>Active</td>
      </tr>
      <tr>
        <td>Vector PDF Engine</td>
        <td>Native backend print to vector PDF stream</td>
        <td>Optimal</td>
      </tr>
      <tr>
        <td>Memory Profiler</td>
        <td>Zero intermediate file writes in-memory stream</td>
        <td>Verified</td>
      </tr>
    </tbody>
  </table>
  <div class="invoice-total">Total Operations: 100% Passed</div>
</body>
</html>
"#;

fn main() -> wry::Result<()> {
  #[cfg(not(target_os = "linux"))]
  let event_loop = tao::event_loop::EventLoop::new();

  let (load_tx, load_rx) = channel();

  let webview = WebViewBuilder::new()
    .with_html(HTML_CONTENT)
    .with_on_page_load_handler(move |event, _| {
      if event == PageLoadEvent::Finished {
        let _ = load_tx.send(());
      }
    })
    .build_headless()?;


  #[cfg(target_os = "linux")]
  {
    while load_rx.try_recv().is_err() {
      while gtk::events_pending() {
        gtk::main_iteration();
      }
      std::thread::sleep(std::time::Duration::from_millis(1));
    }

    let (pdf_tx, pdf_rx) = channel();
    webview.print_to_pdf(PrintToPdfOptions::default(), move |result| {
      let _ = pdf_tx.send(result);
    })?;

    let pdf_bytes = loop {
      while gtk::events_pending() {
        gtk::main_iteration();
      }
      if let Ok(result) = pdf_rx.try_recv() {
        break result?;
      }
      std::thread::sleep(std::time::Duration::from_millis(1));
    };

    assert!(!pdf_bytes.is_empty(), "Generated PDF buffer cannot be empty");
    exit(0);

  }

  #[cfg(not(target_os = "linux"))]
  {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tao::event_loop::ControlFlow;

    let printed = Arc::new(AtomicBool::new(false));
    let printed_clone = printed.clone();
    let webview_ref = Arc::new(webview);
    let webview_for_event = webview_ref.clone();

    event_loop.run(move |_, _, control_flow| {
      *control_flow = ControlFlow::Poll;


      if load_rx.try_recv().is_ok() && !printed_clone.swap(true, Ordering::SeqCst) {
        let _ = webview_for_event.print_to_pdf(PrintToPdfOptions::default(), move |result| {
          let pdf_bytes = result.expect("PDF in-memory generation failed");
          assert!(!pdf_bytes.is_empty(), "Generated PDF buffer cannot be empty");
          exit(0);
        });
      }
    });
  }

  #[allow(unreachable_code)]
  Ok(())
}
