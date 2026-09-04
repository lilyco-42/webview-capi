import { Webview } from "webview";
const w = new Webview({ debug: false });
w.title = "{NAME}";
w.size(1100, 760, 0);
w.navigate("{URL}");
w.run();
w.destroy();
