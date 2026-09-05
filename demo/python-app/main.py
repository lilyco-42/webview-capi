import webview

def main():
    window = webview.create_window(
        title="MC Console",
        url="http://192.168.10.165:8765",
        width=1100,
        height=760
    )
    webview.start(debug=False)

if __name__ == "__main__":
    main()
