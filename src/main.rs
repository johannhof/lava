extern crate markdown;

fn main() {
    println!("Testing markdown integration");
    let text = markdown::to_html("This is a *text*");
    println!("{}", text);
}

