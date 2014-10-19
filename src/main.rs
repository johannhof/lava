extern crate markdown;
extern crate getopts;

use getopts::{optopt,optflag,getopts, usage, short_usage, Matches};
use std::os;
use std::io;
use std::io::fs;
use std::io::File;
use std::io::fs::PathExtensions;

fn build(matches: Matches) {
    let source = match matches.opt_str("s"){
        Some(x) => Path::new(x),
        None => Path::new("./")
    };

    let dest = match matches.opt_str("d"){
        Some(x) => Path::new(x),
        None => Path::new("./_site/")
    };

    println!("Generating from {} to {}", source.display(), dest.display());

    fs::mkdir(&dest, io::USER_RWX);

    match fs::walk_dir(&source.join("_pages")) {
        Err(why) => println!("! {}", why.detail),
        Ok(mut paths) => for path in paths {
            render_file(&path);
        }
    }

    if source != dest {
        match copy_recursive(&source, &dest, |p| -> bool {
            !p.filename_str().unwrap().starts_with(".") &&
            !(p.path_relative_from(&source).unwrap().as_str().unwrap() == "articles")
        }) {
            Err(why) => println!("! {}", why.detail),
            _ => {}
        }
    }

    println!("Testing markdown integration");
    let text = markdown::to_html("This is a *text*");
    println!("{}", text);

    println!("Done");
}

fn render_file(path : &Path) {
    let mut file = File::open(path);
    let content = match file.read_to_string(){
        Ok(x) => x,
        Err(e) => {
            println!("Could not read file {} : {}", path.display(), e)
            return;
        }
    };
    println!("{}", content);
}

fn copy_recursive(source: &Path, dest: &Path, valid: |&Path| -> bool) -> io::IoResult<()> {
    if source.is_dir() {
        let contents = try!(fs::readdir(source));
        for entry in contents.iter() {
            if entry.is_dir() {
                if valid(entry) {
                    let new_dest = &dest.join(entry.path_relative_from(source).unwrap());
                    fs::mkdir(new_dest, io::USER_RWX);
                    try!(copy_recursive(entry, new_dest, |p| valid(p)));
                }
            } else {
                if valid(entry) {
                    try!(fs::copy(entry, &dest.join(entry.path_relative_from(source).unwrap())));
                }
            }
        }
        Ok(())
    } else {
        Err(io::standard_error(io::InvalidInput))
    }
}

fn main() {

    let opts = [
        optopt("d", "destination", "set destination directory", "NAME"),
        optopt("s", "source", "set source directory", "NAME"),
        optflag("h", "help", "print this help menu")
    ];

    let instructions = "Usage: lava [command] ";

    let args = os::args();
    let program = args[0].clone();

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => { m }
        Err(f) => { fail!(f.to_string()) }
    };

    if matches.opt_present("h") {
        println!("{}", usage(program.as_slice(), opts));
        return;
    }

    let command = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        println!("{}", short_usage(instructions, opts));
        return;
    };

    match command.as_slice()  {
        "build" => build(matches),
        _ => {
            println!("{}", short_usage(instructions, opts));
            return;
        }
    }
}

