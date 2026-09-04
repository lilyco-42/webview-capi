#include "webview.h"
int main(void) {
    webview_t w = webview_create(0, NULL);
    webview_set_title(w, "{NAME}");
    webview_set_size(w, 1100, 760, WEBVIEW_HINT_NONE);
    webview_navigate(w, "{URL}");
    webview_run(w);
    webview_destroy(w);
    return 0;
}
