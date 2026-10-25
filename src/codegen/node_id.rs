use std::fmt::{
    self,
    Display,
    Formatter,
    Write,
};

#[derive(Debug, Copy, Clone)]
pub struct NodeID {
    value: u64,
}

impl NodeID {
    pub fn new(value: u64) -> Self {
        Self { value }
    }
}

const CHARSET: &[u8] =
    b"!#%()*+,-./:;=?@[]^_`{|}~ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
impl Display for NodeID {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut n = self.value;
        write!(f, "\"$")?;
        if n == 0 {
            f.write_char(CHARSET[0] as char)?;
        } else {
            while n > 0 {
                f.write_char(CHARSET[(n % CHARSET.len() as u64) as usize] as char)?;
                n /= CHARSET.len() as u64;
            }
        }
        write!(f, "\"")
    }
}
