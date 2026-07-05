import QtQuick
import QtQuick.Controls
import QtQuick.Window
import QtWebEngine
import QtWebChannel
import opendeck

ApplicationWindow {
    id: window
    visible: true
    width: 1100
    height: 720
    title: qsTr("Rust + qtbridge + QtWebEngine Shell")

    // A simple webchannel to allow Javascript <-> Rust Communication
    WebChannel {
        id: channel
    }

    Component.onCompleted: {
        // Register the backend into the WebChannel so JavaScript can call it
        channel.registerObject("backend", Backend)
    }

    // Every 10 seconds we want to force a garbage collection, to hopefully keep the memory usage down.
    Timer {
        id: forceJavaScriptGC
        interval: 10000
        repeat: true
        running: true
        onTriggered: webView.runJavaScript(`if (window.gc) window.gc();`)
    }

    WebEngineView {
        id: webView
        anchors.fill: parent
        webChannel: channel
        url: "__WEB_ROOT__"

        // Log JS Console messages to the terminal
        onJavaScriptConsoleMessage: function (level, message, lineNumber, sourceID) {
            console.log("[JS]", message, "(" + sourceID + ":" + lineNumber + ")")
        }

        settings.localContentCanAccessRemoteUrls: true
        settings.localContentCanAccessFileUrls: true
    }
}
