using Microsoft.Web.WebView2.WinForms;
using System.Windows.Forms;
class Program {
    [STAThread]
    static void Main() {
        var form = new Form();
        form.Text = "{NAME}";
        form.Size = new System.Drawing.Size(1100, 760);
        var web = new WebView2();
        web.Dock = DockStyle.Fill;
        web.Source = new Uri("{URL}");
        form.Controls.Add(web);
        Application.Run(form);
    }
}
