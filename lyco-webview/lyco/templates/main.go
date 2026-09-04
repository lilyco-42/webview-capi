package main
import "github.com/webview/webview"
func main() {
    w := webview.New(false)
    defer w.Destroy()
    w.SetTitle("{NAME}")
    w.SetSize(1100, 760, webview.HintNone)
    w.Navigate("{URL}")
    w.Run()
}
