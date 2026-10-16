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
  <title>Benchmark PDF File Document</title>
  <style>
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      margin: 40px;
      color: #333;
      line-height: 1.6;
    }
    h1 {
      color: #2c3e50;
      border-bottom: 2px solid #27ae60;
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
    .status-badge {
      display: inline-block;
      padding: 4px 8px;
      background-color: #2ecc71;
      color: white;
      border-radius: 4px;
      font-size: 0.9em;
    }
  </style>
</head>
<body>
  <h1>Direct-to-File PDF Benchmark Report</h1>
  <p>Benchmarking native off-screen rendering combined with atomic filesystem writer.</p>
  <table class="header-table">
    <thead>
      <tr>
        <th>Pipeline Stage</th>
        <th>Mechanism</th>
        <th>State</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>HTML Parsing &amp; Layout</td>
        <td>Headless WebKit / Blink Engine</td>
        <td><span class="status-badge">Loaded</span></td>
      </tr>
      <tr>
        <td>Vector Rasterization</td>
        <td>Native OS Print Spooler / Skia / Cairo</td>
        <td><span class="status-badge">Printed</span></td>
      </tr>
      <tr>
        <td>Filesystem Write</td>
        <td>Direct atomic file I/O</td>
        <td><span class="status-badge">Saved</span></td>
      </tr>
    </tbody>
  </table>
</body>
</html>
"#;

fn main() -> wry::Result<()> {
  #[cfg(not(target_os = "linux"))]
  let event_loop = tao::event_loop::EventLoop::new();

  let temp_path = std::env::temp_dir().join(format!("bench_wry_pdf_file_{}.pdf", std::process::id()));
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
    let file_dest = temp_path.clone();
    webview.print_to_pdf_file(&file_dest, PrintToPdfOptions::default(), move |result| {
      let _ = pdf_tx.send(result);
    })?;

    loop {
      while gtk::events_pending() {
        gtk::main_iteration();
      }
      if let Ok(result) = pdf_rx.try_recv() {
        result?;
        break;
      }
      std::thread::sleep(std::time::Duration::from_millis(1));
    }

    let file_len = std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0);
    assert!(file_len > 0, "PDF output file must exist and have non-zero size");
    let _ = std::fs::remove_file(&temp_path);
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
        let target_path = temp_path.clone();
        let path_clone = target_path.clone();
        let _ = webview_for_event.print_to_pdf_file(&target_path, PrintToPdfOptions::default(), move |result| {
          result.expect("PDF file write failed");
          let file_len = std::fs::metadata(&path_clone).map(|m| m.len()).unwrap_or(0);
          assert!(file_len > 0, "PDF output file must exist and have non-zero size");
          let _ = std::fs::remove_file(&path_clone);
          exit(0);
        });

      }
    });
  }

  #[allow(unreachable_code)]
  Ok(())
}
