// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

@file:Suppress("unused", "SetJavaScriptEnabled")

package {{package}}

import android.annotation.SuppressLint
import android.webkit.*
import android.content.Context
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import kotlin.collections.Map

@SuppressLint("RestrictedApi")
class RustWebView(context: Context, val initScripts: Array<String>, val id: String): WebView(context) {
    val isDocumentStartScriptEnabled: Boolean

    init {
        settings.javaScriptEnabled = true
        settings.domStorageEnabled = true
        settings.setGeolocationEnabled(true)
        settings.databaseEnabled = true
        settings.mediaPlaybackRequiresUserGesture = false
        settings.javaScriptCanOpenWindowsAutomatically = true

        if (WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) {
            isDocumentStartScriptEnabled = true
            for (script in initScripts) {
                WebViewCompat.addDocumentStartJavaScript(this, script, setOf("*"));
            }
        } else {
          isDocumentStartScriptEnabled = false
        }

        {{class-init}}
    }

    fun loadUrlMainThread(url: String) {
        post {
          loadUrl(url)
        }
    }

    fun loadUrlMainThread(url: String, additionalHttpHeaders: Map<String, String>) {
        post {
          loadUrl(url, additionalHttpHeaders)
        }
    }

    override fun loadUrl(url: String) {
        if (!Rust.shouldOverride(id, url)) {
            super.loadUrl(url);
        }
    }

    override fun loadUrl(url: String, additionalHttpHeaders: Map<String, String>) {
        if (!Rust.shouldOverride(id, url)) {
            super.loadUrl(url, additionalHttpHeaders);
        }
    }

    fun loadHTMLMainThread(html: String) {
        post {
          // A null base URL gives about:blank, which IPC accepts as a request URI.
          super.loadDataWithBaseURL(null, html, "text/html", "UTF-8", null)
        }
    }

    fun evalScript(id: Int, script: String) {
        post {
            super.evaluateJavascript(script) { result ->
                Rust.onEval(this.id, id, result)
            }
        }
    }

    fun printToPdf(id: Int, landscape: Boolean, pageWidth: Double, pageHeight: Double, marginTop: Double, marginBottom: Double, marginLeft: Double, marginRight: Double) {
        post {
            var readFd: android.os.ParcelFileDescriptor? = null
            var writeFd: android.os.ParcelFileDescriptor? = null
            if (android.os.Build.VERSION.SDK_INT < android.os.Build.VERSION_CODES.KITKAT) {
                Rust.onPdf(this.id, id, null)
                return@post
            }
            try {
                val adapter = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.LOLLIPOP) {
                    this.createPrintDocumentAdapter("wry_pdf")
                } else {
                    this.createPrintDocumentAdapter()
                }
                val mediaSize = if (pageWidth > 0 && pageHeight > 0) {
                    val wMils = (pageWidth * 1000).toInt()
                    val hMils = (pageHeight * 1000).toInt()
                    android.print.PrintAttributes.MediaSize("custom", "Custom", wMils, hMils)
                } else if (landscape) {
                    android.print.PrintAttributes.MediaSize.ISO_A4.asLandscape()
                } else {
                    android.print.PrintAttributes.MediaSize.ISO_A4.asPortrait()
                }

                val margins = android.print.PrintAttributes.Margins(
                    (marginLeft * 1000).toInt(),
                    (marginTop * 1000).toInt(),
                    (marginRight * 1000).toInt(),
                    (marginBottom * 1000).toInt()
                )

                val attributes = android.print.PrintAttributes.Builder()
                    .setMediaSize(mediaSize)
                    .setResolution(android.print.PrintAttributes.Resolution("pdf", "pdf", 300, 300))
                    .setMinMargins(margins)
                    .build()

                val pipe = android.os.ParcelFileDescriptor.createPipe()
                readFd = pipe[0]
                writeFd = pipe[1]
                val currentReadFd = readFd
                val currentWriteFd = writeFd

                adapter.onLayout(null, attributes, android.os.CancellationSignal(), object : android.print.PrintDocumentAdapter.LayoutResultCallback() {
                    override fun onLayoutFinished(info: android.print.PrintDocumentInfo?, changed: Boolean) {
                        adapter.onWrite(arrayOf(android.print.PageRange.ALL_PAGES), currentWriteFd, android.os.CancellationSignal(), object : android.print.PrintDocumentAdapter.WriteResultCallback() {
                            override fun onWriteFinished(pages: Array<out android.print.PageRange>?) {
                                try { currentWriteFd.close() } catch (_: Exception) {}
                                Thread {
                                    try {
                                        val input = java.io.FileInputStream(currentReadFd.fileDescriptor)
                                        val bytes = input.readBytes()
                                        try { currentReadFd.close() } catch (_: Exception) {}
                                        Rust.onPdf(this@RustWebView.id, id, bytes)
                                    } catch (e: Exception) {
                                        try { currentReadFd.close() } catch (_: Exception) {}
                                        Rust.onPdf(this@RustWebView.id, id, null)
                                    }
                                }.start()
                            }

                            override fun onWriteFailed(error: CharSequence?) {
                                try { currentWriteFd.close() } catch (_: Exception) {}
                                try { currentReadFd.close() } catch (_: Exception) {}
                                Rust.onPdf(this@RustWebView.id, id, null)
                            }
                        })
                    }

                    override fun onLayoutFailed(error: CharSequence?) {
                        try { currentWriteFd.close() } catch (_: Exception) {}
                        try { currentReadFd.close() } catch (_: Exception) {}
                        Rust.onPdf(this@RustWebView.id, id, null)
                    }
                }, android.os.Bundle())
            } catch (e: Exception) {
                try { writeFd?.close() } catch (_: Exception) {}
                try { readFd?.close() } catch (_: Exception) {}
                Rust.onPdf(this.id, id, null)
            }
        }
    }

    fun clearAllBrowsingData() {
        try {
            super.getContext().deleteDatabase("webviewCache.db")
            super.getContext().deleteDatabase("webview.db")
            super.clearCache(true)
            super.clearHistory()
            super.clearFormData()
        } catch (ex: Exception) {
            Logger.error("Unable to create temporary media capture file: " + ex.message)
        }
    }

    fun getCookies(url: String): String {
        val cookieManager = CookieManager.getInstance()
        return cookieManager.getCookie(url)
    }

    {{class-extension}}
}
