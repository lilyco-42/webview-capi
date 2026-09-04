import javafx.application.Application;
import javafx.scene.Scene;
import javafx.scene.web.WebView;
import javafx.stage.Stage;
public class Main extends Application {
    public void start(Stage stage) {
        WebView web = new WebView();
        web.getEngine().load("{URL}");
        stage.setScene(new Scene(web, 1100, 760));
        stage.setTitle("{NAME}");
        stage.show();
    }
    public static void main(String[] args) { launch(args); }
}
