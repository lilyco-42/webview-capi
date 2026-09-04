use webview::WebView;
fn main() {
    let w = WebView::create(true);
    w.set_title("{NAME}");
    w.set_size(1100, 760, 0);
    w.navigate("{URL}");
    w.run();
    w.destroy();
}
