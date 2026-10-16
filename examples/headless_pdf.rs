// Copyright 2020-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::sync::{Arc, Mutex};
use wry::{PrintToPdfOptions, WebViewBuilder};

fn main() -> wry::Result<()> {
  println!("Starting headless PDF generation test...");

  let html = r#"
    <!DOCTYPE html>
    <html>
      <head>
        <meta charset="utf-8">
        <style>
          @page {
            size: A4 portrait;
            margin: 0;
          }
          body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            background: #ffffff;
            color: #111827;
            padding: 40px;
            margin: 0;
          }
          .header {
            border-bottom: 2px solid #3b82f6;
            padding-bottom: 16px;
            margin-bottom: 24px;
          }
          h1 {
            color: #1e3a8a;
            margin: 0 0 8px 0;
            font-size: 28px;
          }
          .subtitle {
            color: #6b7280;
            font-size: 14px;
          }
          .badge {
            display: inline-block;
            background: #dbeafe;
            color: #1d4ed8;
            padding: 4px 12px;
            border-radius: 9999px;
            font-size: 12px;
            font-weight: 600;
            margin-top: 8px;
          }
          .card {
            background: #f9fafb;
            border: 1px solid #e5e7eb;
            border-radius: 8px;
            padding: 20px;
            margin-bottom: 20px;
          }
          table {
            width: 100%;
            border-collapse: collapse;
            margin-top: 16px;
          }
          th, td {
            text-align: left;
            padding: 10px 12px;
            border-bottom: 1px solid #e5e7eb;
          }
          th {
            background: #f3f4f6;
            font-weight: 600;
            font-size: 13px;
          }
          td {
            font-size: 13px;
          }
          .footer {
            margin-top: 32px;
            font-size: 11px;
            color: #9ca3af;
            text-align: center;
          }
        </style>
      </head>
      <body>
        <div class="header">
          <h1>Wry Headless PDF Report</h1>
          <div class="subtitle">Generated natively via Wry build_headless() & print_to_pdf()</div>
          <span class="badge">In-Memory Vector PDF</span>
        </div>
        <div class="card">
          <h3>Target Platform Verification</h3>
          <table>
            <thead>
              <tr>
                <th>Platform</th>
                <th>Engine</th>
                <th>Headless Architecture</th>
                <th>Vector Memory Stream</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Linux</td>
                <td>WebKitGTK 4.1</td>
                <td>OffscreenWindow / Cairo</td>
                <td>/dev/shm &rarr; Vec&lt;u8&gt;</td>
              </tr>
              <tr>
                <td>macOS / iOS</td>
                <td>WKWebView</td>
                <td>Detached View</td>
                <td>createPDFWithConfiguration &rarr; NSData &rarr; Vec&lt;u8&gt;</td>
              </tr>
              <tr>
                <td>Windows</td>
                <td>WebView2</td>
                <td>Hidden WS_POPUP HWND</td>
                <td>PrintToPdfStream &rarr; IStream &rarr; Vec&lt;u8&gt;</td>
              </tr>
              <tr>
                <td>Android</td>
                <td>Android WebView</td>
                <td>Detached RustWebView</td>
                <td>PrintDocumentAdapter &rarr; Pipe &rarr; Vec&lt;u8&gt;</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="footer">
          Generated automatically with Wry 0.57 (Forked for Cross-Platform Headless PDF)
        </div>
      </body>
    </html>
  "#;

  let (tx, rx) = std::sync::mpsc::channel();
  let tx_clone = tx.clone();

  let webview = WebViewBuilder::new()
    .with_html(html)
    .with_on_page_load_handler(move |event, url| {
      if event == wry::PageLoadEvent::Finished {
        println!("Page loaded successfully: {url}");
        let _ = tx_clone.send(());
      }
    })
    .build_headless()?;

  println!("Headless webview created successfully! Waiting for page to load...");

  #[cfg(target_os = "linux")]
  {
    while rx.try_recv().is_err() {
      while gtk::events_pending() {
        gtk::main_iteration();
      }
      std::thread::sleep(std::time::Duration::from_millis(10));
    }
  }
  #[cfg(not(target_os = "linux"))]
  {
    rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
  }

  println!("Page finished loading. Printing document directly to in-memory PDF...");

  let pdf_result = Arc::new(Mutex::new(None));
  let pdf_res_clone = pdf_result.clone();

  let options = PrintToPdfOptions {
    landscape: false,
    page_width: Some(8.27),
    page_height: Some(11.69),
    margin_top: 0.4,
    margin_bottom: 0.4,
    margin_left: 0.4,
    margin_right: 0.4,
    print_background: true,
    scale: 1.0,
  };

  webview.print_to_pdf(options, move |result| {
    *pdf_res_clone.lock().unwrap() = Some(result);
  })?;

  #[cfg(target_os = "linux")]
  {
    while pdf_result.lock().unwrap().is_none() {
      while gtk::events_pending() {
        gtk::main_iteration();
      }
      std::thread::sleep(std::time::Duration::from_millis(10));
    }
  }

  let final_result = pdf_result.lock().unwrap().take().unwrap();
  match final_result {
    Ok(bytes) => {
      println!(
        "SUCCESS: Generated PDF in-memory! Size: {} bytes ({} KB)",
        bytes.len(),
        bytes.len() / 1024
      );
      let output_file = "headless_output.pdf";
      std::fs::write(output_file, &bytes)?;
      println!("Saved vector PDF to: {output_file}");
    }
    Err(e) => {
      eprintln!("ERROR: Failed to generate PDF: {e}");
      return Err(e);
    }
  }

  Ok(())
}
