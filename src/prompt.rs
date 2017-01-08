use std::io;
use std::io::{Read, Write};
use std::process;

pub trait Prompt {
    fn do_prompt(&self) -> Result<String, io::Error>;
}

pub struct DmenuPrompt<'a> {
    selections: &'a [String],
    args: &'a [&'a str],
}

impl<'a> DmenuPrompt<'a> {
    pub fn new(selections: &'a [String], args: &'a [&'a str]) -> Self {
        DmenuPrompt {
            selections: selections,
            args: args,
        }
    }
}

impl<'a> Prompt for DmenuPrompt<'a> {
    fn do_prompt(&self) -> Result<String, io::Error> {
        let mut child = try!(process::Command::new("dmenu")
            .args(self.args)
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .spawn());
        for c in self.selections {
            try!(writeln!(child.stdin.as_mut().unwrap(), "{}", c));
        }
        try!(child.wait());
        let mut result = String::new();
        try!(child.stdout.unwrap().read_to_string(&mut result));
        Ok(result.trim().into())
    }
}