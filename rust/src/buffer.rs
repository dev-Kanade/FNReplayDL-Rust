pub struct ReplayBuffer {
    data: Vec<u8>,
}

impl ReplayBuffer {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn write_byte(&mut self, val: u8) {
        self.data.push(val);
    }

    pub fn write_int32(&mut self, val: i32, offset: Option<usize>) {
        let bytes = val.to_le_bytes();
        if let Some(off) = offset {
            if off + 4 <= self.data.len() {
                self.data[off..off + 4].copy_from_slice(&bytes);
            }
        } else {
            self.data.extend_from_slice(&bytes);
        }
    }

    pub fn write_int64(&mut self, val: i64) {
        self.data.extend_from_slice(&val.to_le_bytes());
    }

    pub fn write_bytes(&mut self, b: &[u8]) {
        self.data.extend_from_slice(b);
    }

    pub fn write_string(&mut self, s: &str) {
        let b = s.as_bytes();
        self.write_int32((b.len() + 1) as i32, None);
        self.write_bytes(b);
        self.write_byte(0);
    }

    pub fn write_array<T, F>(&mut self, arr: &[T], mut f: F)
    where
        F: FnMut(&mut Self, &T),
    {
        self.write_int32(arr.len() as i32, None);
        for item in arr {
            f(self, item);
        }
    }

    pub fn length(&self) -> usize {
        self.data.len()
    }

    pub fn get_data(self) -> Vec<u8> {
        self.data
    }
}
