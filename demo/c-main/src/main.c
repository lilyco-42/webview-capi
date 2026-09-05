#include "webview.h"
#include <stddef.h>   // NULL

int main(void) {
    webview_t w = webview_create(0, NULL);
    webview_set_title(w, "MC Console");
    webview_set_size(w, 1100, 760, WEBVIEW_HINT_NONE);
    webview_navigate(w, "http://192.168.10.165:8765");
    webview_run(w);
    webview_destroy(w);
    return 0;
}
