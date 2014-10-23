#![feature(phase)]
#[phase(plugin)]
extern crate regex_macros;
extern crate regex;
extern crate markdown;
extern crate getopts;

use getopts::{optopt,optflag,getopts, usage, short_usage, Matches};
use std::os;
use std::io;
use std::str;
use std::str::MaybeOwned;
use std::io::fs;
use std::io::File;
use std::io::fs::PathExtensions;
use std::collections::HashMap;
use regex::Regex;

static FRONT_MATTER : Regex = regex!(r"^-{3,}\n(?P<data>(.*\n)*)-{3,}");
static YAML : Regex = regex!(r"(?P<key>.*?)\s*:\s*(?P<value>.*)");
static PARTIAL : Regex = regex!(r"\{\{p\s(?P<name>.*?)\}\}");

struct Page {
    path : Path,
    template : String,
    keys : HashMap<String, String>
}

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

    fs::mkdir(&dest, io::USER_READ);

    let mut partials = HashMap::new();

    // get partials
    let partial_path = fs::readdir(&source.join("_templates/partials")).unwrap();
    for partial in partial_path.iter() {
        let mut file = File::open(partial);
        match file.read_to_string(){
            Ok(x) => {
                partials.insert(partial.filename_str().unwrap().to_string(), x);
            }
            Err(e) => {
                println!("Could not read file {} : {}", partial.display(), e);
            }
        };
    }

    let mut templates = HashMap::new();

    // get templates
    let template_path = fs::readdir(&source.join("_templates")).unwrap();
    for template in template_path.iter() {
        if template.is_file(){
            let mut file = File::open(template);
            match file.read_to_string(){
                Ok(content) => {
                    let mut text = content.to_string();
                    for caps in PARTIAL.captures_iter(content.as_slice()) {
                        match partials.find(&(caps.name("name").to_string())) {
                            Some(x) => text = text.replace(caps.at(0), x.as_slice()),
                            None => println!("WARNING: {} not found.", caps.name("name"))
                        };
                    }
                    templates.insert(template.filename_str().unwrap().to_string(), text);
                }
                Err(e) => {
                    println!("Could not read file {} : {}", template.display(), e);
                }
            };
        }
    }

    // copy everything
    if source != dest {
        match copy_recursive(&source, &dest, |p| -> bool {
            !p.filename_str().unwrap().starts_with(".")
            && p != &dest
            && p.path_relative_from(&source).unwrap().as_str().unwrap() != "_pages"
            && p.path_relative_from(&source).unwrap().as_str().unwrap() != "_templates"
        }) {
            Err(why) => println!("! {}", why.detail),
            _ => {}
        }
    }

    // generate pages
    match fs::walk_dir(&source.join("_pages")) {
        Err(why) => println!("! {}", why.detail),
        Ok(mut paths) => for path in paths {
            if path.is_file() {
                match get_page(&path){
                    Ok(page) => {
                        let new_dest = &dest.join(path.path_relative_from(&source.join("_pages")).unwrap());
                        fs::mkdir_recursive(&new_dest.dir_path(), io::USER_READ);
                        match templates.find(&page.template) {
                            Some(template) => {
                                match File::create(new_dest).write_str(create_content(template, &page.keys).as_slice()){
                                    Ok(_) => println!("Generated {}", new_dest.display()),
                                    Err(x) => println!("{}", x)
                                }
                            }
                            None => println!("Template {} not found", page.template)
                        };
                    },
                    Err(e) => println!("{}: {}", path.display(), e)
                };
            }
        }
    };

    println!("Done");
}

fn create_content(template : &String, keys : &HashMap<String, String>) -> String{
    // TODO change to proper template lang
    let mut result = template.to_string();
    for (key, value) in keys.iter() {
        let mut s = String::from_str("{{= ");
        s.push_str(key.as_slice());
        s.push_str("}}");
        result = result.replace(s.as_slice(), value.as_slice());
    }
    result
}

fn get_page(path : &Path) -> Result<Page, &'static str>{

    let mut file = File::open(path);
    let content = match file.read_to_string(){
        Ok(x) => x,
        Err(e) => return Err("Could not read file")
    };

    match FRONT_MATTER.captures(content.as_slice()){
        Some(fm) => {
            let mut keys : HashMap<String, String> = HashMap::new();
            for caps in YAML.captures_iter(fm.name("data")) {
                keys.insert(caps.name("key").to_string(), caps.name("value").to_string());
            }
            keys.insert("content".to_string(), markdown::to_html(FRONT_MATTER.replace(content.as_slice(), "").as_slice()));

            let template = try!(keys.find(&"template".to_string()).ok_or("Page needs to define a template"));
            Ok(Page {
                path : path.clone(),
                template : template.to_string(),
                keys : keys.clone()
            })
        },
        None => Err("Page needs to define a template")
    }
}

fn copy_recursive(source: &Path, dest: &Path, valid: |&Path| -> bool) -> io::IoResult<()> {
    if source.is_dir() {
        let contents = try!(fs::readdir(source));
        for entry in contents.iter() {
            if entry.is_dir() {
                if valid(entry) {
                    let new_dest = &dest.join(entry.path_relative_from(source).unwrap());
                    fs::mkdir(new_dest, io::USER_READ);
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

