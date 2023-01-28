use owo_colors::OwoColorize;
use std::fmt::Display;
use ubyte::ByteUnit;

#[derive(Default)]
pub(crate) struct ParentPrintBuffer {
    // invariant: if parent is not empty, then buffer is not empty either.
    parent: String,
    buffer: Vec<(String, Option<String>, ByteUnit)>,
    group: usize,
}

impl ParentPrintBuffer {
    pub fn new(group: usize) -> Self {
        Self {
            group,
            ..Default::default()
        }
    }

    pub fn push(&mut self, path: String, username: Option<String>, size: ByteUnit) {
        if self.group == 0 {
            print(&path, username, size, "");
            return;
        }
        let given_parent = parent_of(&path).to_string();
        if self.parent.is_empty() {
            self.parent = given_parent;
            self.buffer.push((path, username, size));
            return;
        }
        if self.parent == given_parent {
            self.buffer.push((path, username, size))
        } else {
            self.flush(username)
        }
    }

    pub fn flush(&mut self, username: Option<String>) {
        if self.group == 0 {
            self.print_all_and_drain();
            return;
        }
        if self.buffer.len() >= self.group {
            print(&self.parent, username, self.total_size(), " (grouped)");
            self.parent.clear();
            self.buffer.clear();
        } else {
            self.print_all_and_drain()
        }
    }

    fn total_size(&self) -> ByteUnit {
        self.buffer
            .iter()
            .map(|(_p, _u, s)| s.clone())
            .fold(ByteUnit::Byte(0), |acc, x| acc + x)
    }

    fn print_all_and_drain(&mut self) {
        self.parent.clear();
        for (path, username, size) in self.buffer.drain(..) {
            print(&path, username, size, "")
        }
    }
}

fn parent_of(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _after)| parent)
        .unwrap_or(path)
}

fn print(path: impl Display, username: Option<String>, size: ByteUnit, suffix: impl Display) {
    let suffix = suffix.dimmed();
    if let Some(user) = username {
        println!("{path} {user} {size}{suffix}")
    } else {
        println!("{path} {size}{suffix}")
    }
}
