//! CPython version-2 string seeding and MT19937 randrange compatibility.
use sha2::{Digest, Sha512};
pub struct Random {
    state: [u32; 624],
    index: usize,
}
impl Random {
    pub fn new(seed: &str) -> Self {
        let mut bytes = seed.as_bytes().to_vec();
        bytes.extend(Sha512::digest(seed.as_bytes()));
        let mut key = vec![];
        for part in bytes.rchunks(4) {
            let mut word = 0u32;
            for byte in part {
                word = (word << 8) | u32::from(*byte);
            }
            key.push(word);
        }
        let mut rng = Self {
            state: [0; 624],
            index: 624,
        };
        rng.state[0] = 19650218;
        for i in 1..624 {
            rng.state[i] = 1812433253u32
                .wrapping_mul(rng.state[i - 1] ^ (rng.state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        let (mut i, mut j) = (1, 0);
        for _ in 0..624.max(key.len()) {
            rng.state[i] = (rng.state[i]
                ^ (rng.state[i - 1] ^ (rng.state[i - 1] >> 30)).wrapping_mul(1664525))
            .wrapping_add(key[j])
            .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= 624 {
                rng.state[0] = rng.state[623];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..623 {
            rng.state[i] = (rng.state[i]
                ^ (rng.state[i - 1] ^ (rng.state[i - 1] >> 30)).wrapping_mul(1566083941))
            .wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                rng.state[0] = rng.state[623];
                i = 1;
            }
        }
        rng.state[0] = 0x80000000;
        rng
    }
    fn next(&mut self) -> u32 {
        if self.index >= 624 {
            for i in 0..624 {
                let y = (self.state[i] & 0x80000000) | (self.state[(i + 1) % 624] & 0x7fffffff);
                self.state[i] = self.state[(i + 397) % 624]
                    ^ (y >> 1)
                    ^ if y & 1 != 0 { 0x9908b0df } else { 0 };
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680;
        y ^= (y << 15) & 0xefc60000;
        y ^ (y >> 18)
    }
    pub fn number(&mut self) -> u32 {
        loop {
            let n = self.next() >> 12;
            if n < 899999 {
                return 100000 + n;
            }
        }
    }
}
