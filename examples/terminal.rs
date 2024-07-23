use conpty_sample_rs::terminal::Terminal;

fn main() {
    unsafe {
        Terminal::run("powershell");
    }
}
