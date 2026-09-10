use std::io;

pub trait Opener {
    fn open(&self, url: &str) -> io::Result<()>;
    fn copy(&self, text: &str) -> io::Result<()>;
}
