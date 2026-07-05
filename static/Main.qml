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

    WebEngineView {
        id: webView
        anchors.fill: parent
        webChannel: channel

        Component.onCompleted: {
            webView.url = "http://127.0.0.1:5173/"
        }

        // Log JS Console messages to the terminal
        onJavaScriptConsoleMessage: function (level, message, lineNumber, sourceID) {
            console.log("[JS]", message, "(" + sourceID + ":" + lineNumber + ")")
        }

        settings {
            localContentCanAccessRemoteUrls: false
            localContentCanAccessFileUrls: true
        }
    }
}
