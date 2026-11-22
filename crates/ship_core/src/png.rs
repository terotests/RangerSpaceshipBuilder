// SPDX-License-Identifier: MIT
//! A PNG writer for RGB images: zlib with deflate's fixed Huffman codes and
//! LZ77 matches (hash chains), CRC-32 and Adler-32 by arithmetic, so every
//! Ranger target writes the same bytes.

use ranger::prelude::*;

/// Writes bits least significant first, as deflate wants, into a buffer
/// sized up front (a buffer that grows copies itself on some targets).
struct Bits {
    out: Vec<u8>,
    pos: int,
    acc: int,
    n: int,
}

impl Bits {
    fn new(capacity: int) -> Bits {
        Bits { out: vec![0u8; capacity as usize], pos: 0, acc: 0, n: 0 }
    }
    /// `count` bits of `value`, low bit first.
    fn put(&mut self, value: int, count: int) {
        let mut v = value;
        let mut k: int = 0;
        while k < count {
            if v % 2 == 1 {
                self.acc += pow2(self.n);
            }
            v = v / 2;
            self.n += 1;
            if self.n == 8 {
                self.out[self.pos as usize] = self.acc as u8;
                self.pos += 1;
                self.acc = 0;
                self.n = 0;
            }
            k += 1;
        }
    }
    /// A Huffman code: `count` bits of `code`, high bit first.
    fn code(&mut self, code: int, count: int) {
        let mut k = count - 1;
        while k >= 0 {
            let bit = (code / pow2(k)) % 2;
            self.put(bit, 1);
            k -= 1;
        }
    }
    fn finish(&mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out[self.pos as usize] = self.acc as u8;
            self.pos += 1;
            self.acc = 0;
            self.n = 0;
        }
        self.out[0..(self.pos as usize)].to_vec()
    }
}

fn pow2(k: int) -> int {
    let mut v: int = 1;
    let mut i: int = 0;
    while i < k {
        v = v * 2;
        i += 1;
    }
    v
}

fn literal(b: &mut Bits, c: int) {
    if c < 144 {
        b.code(48 + c, 8);
    } else if c < 256 {
        b.code(400 + c - 144, 9);
    } else if c < 280 {
        b.code(c - 256, 7);
    } else {
        b.code(192 + c - 280, 8);
    }
}

const LEN_BASE: [int; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEN_EXTRA: [int; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE: [int; 30] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
const DIST_EXTRA: [int; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

fn match_out(b: &mut Bits, len: int, dist: int) {
    let mut i: int = 28;
    while LEN_BASE[i as usize] > len {
        i -= 1;
    }
    literal(b, 257 + i);
    b.put(len - LEN_BASE[i as usize], LEN_EXTRA[i as usize]);
    let mut d: int = 29;
    while DIST_BASE[d as usize] > dist {
        d -= 1;
    }
    b.code(d, 5);
    b.put(dist - DIST_BASE[d as usize], DIST_EXTRA[d as usize]);
}

/// Raw deflate of `data`: one final block with the fixed codes.
pub fn deflate(data: &Vec<u8>) -> Vec<u8> {
    let n = data.len() as int;
    let mut b = Bits::new(n * 9 / 8 + 64);
    b.put(1, 1);
    b.put(1, 2);
    let hsize: int = 32768;
    let mut head: Vec<int> = vec![-1; hsize as usize];
    let mut prev: Vec<int> = vec![-1; n.max(1) as usize];
    let mut i: int = 0;
    while i < n {
        let mut best_len: int = 0;
        let mut best_dist: int = 0;
        if i + 2 < n {
            let h = ((data[i as usize] as int) * 1024 + (data[(i + 1) as usize] as int) * 32 + (data[(i + 2) as usize] as int)) % hsize;
            let mut cand = head[h as usize];
            let mut tries: int = 0;
            while cand >= 0 && i - cand <= 32768 && tries < 24 {
                let mut l: int = 0;
                while l < 258 && i + l < n && data[(cand + l) as usize] == data[(i + l) as usize] {
                    l += 1;
                }
                if l > best_len {
                    best_len = l;
                    best_dist = i - cand;
                    if l == 258 {
                        break;
                    }
                }
                cand = prev[cand as usize];
                tries += 1;
            }
            prev[i as usize] = head[h as usize];
            head[h as usize] = i;
        }
        if best_len >= 3 {
            match_out(&mut b, best_len, best_dist);
            // the skipped positions still go into the chains
            let mut k: int = 1;
            while k < best_len {
                let j = i + k;
                if j + 2 < n {
                    let h = ((data[j as usize] as int) * 1024 + (data[(j + 1) as usize] as int) * 32 + (data[(j + 2) as usize] as int)) % hsize;
                    prev[j as usize] = head[h as usize];
                    head[h as usize] = j;
                }
                k += 1;
            }
            i += best_len;
        } else {
            literal(&mut b, data[i as usize] as int);
            i += 1;
        }
    }
    literal(&mut b, 256);
    b.finish()
}

/// The xor of two 32-bit numbers, in 16-bit halves (JavaScript's `^` works
/// on signed 32-bit numbers, so a value of 2^31 or more would turn negative).
fn xor32(a: int, b: int) -> int {
    ((a / 65536) ^ (b / 65536)) * 65536 + ((a % 65536) ^ (b % 65536))
}

fn crc_table() -> Vec<int> {
    let mut t: Vec<int> = Vec::new();
    let mut n: int = 0;
    while n < 256 {
        let mut c = n;
        let mut k: int = 0;
        while k < 8 {
            if c % 2 == 1 {
                c = xor32(3988292384, c / 2);
            } else {
                c = c / 2;
            }
            k += 1;
        }
        t.push(c);
        n += 1;
    }
    t
}

fn crc32(t: &Vec<int>, data: &Vec<u8>, from: int, to: int) -> int {
    let mut c: int = 4294967295;
    let mut i = from;
    while i < to {
        let idx = xor32(c, data[i as usize] as int) % 256;
        c = xor32(t[idx as usize], c / 256);
        i += 1;
    }
    xor32(c, 4294967295)
}

fn set_u32(out: &mut Vec<u8>, at: int, v: int) {
    out[at as usize] = ((v / 16777216) % 256) as u8;
    out[(at + 1) as usize] = ((v / 65536) % 256) as u8;
    out[(at + 2) as usize] = ((v / 256) % 256) as u8;
    out[(at + 3) as usize] = (v % 256) as u8;
}

fn copy_in(out: &mut Vec<u8>, at: int, src: &Vec<u8>) {
    let mut i: int = 0;
    let n = src.len() as int;
    while i < n {
        out[(at + i) as usize] = src[i as usize];
        i += 1;
    }
}

/// Writes a chunk at `at`; where the next one starts.
fn chunk(out: &mut Vec<u8>, t: &Vec<int>, at: int, kind: &str, data: &Vec<u8>) -> int {
    let n = data.len() as int;
    set_u32(out, at, n);
    let kb = kind.as_bytes();
    let mut i: int = 0;
    while i < 4 {
        out[(at + 4 + i) as usize] = kb[i as usize];
        i += 1;
    }
    copy_in(out, at + 8, data);
    let crc = crc32(t, out, at + 4, at + 8 + n);
    set_u32(out, at + 8 + n, crc);
    at + 12 + n
}

/// A PNG of an RGB image, `w` × `h`, rows top first.
pub fn write_rgb(w: int, h: int, rgb: &Vec<u8>) -> Vec<u8> {
    // each row: filter 1 (Sub), then the differences
    let row = w * 3;
    let mut raw = vec![0u8; ((row + 1) * h) as usize];
    let mut y: int = 0;
    while y < h {
        let o = y * (row + 1);
        raw[o as usize] = 1;
        let mut x: int = 0;
        while x < row {
            let cur = rgb[(y * row + x) as usize] as int;
            let left = if x >= 3 { rgb[(y * row + x - 3) as usize] as int } else { 0 };
            raw[(o + 1 + x) as usize] = ((cur - left + 256) % 256) as u8;
            x += 1;
        }
        y += 1;
    }
    let d = deflate(&raw);
    let dn = d.len() as int;
    // zlib: header, the deflate stream, Adler-32
    let mut z = vec![0u8; (dn + 6) as usize];
    z[0] = 120;
    z[1] = 1;
    copy_in(&mut z, 2, &d);
    let mut a: int = 1;
    let mut b: int = 0;
    let rn = raw.len() as int;
    let mut i: int = 0;
    while i < rn {
        a = (a + (raw[i as usize] as int)) % 65521;
        b = (b + a) % 65521;
        i += 1;
    }
    set_u32(&mut z, dn + 2, b * 65536 + a);
    let t = crc_table();
    let mut ihdr = vec![0u8; 13];
    set_u32(&mut ihdr, 0, w);
    set_u32(&mut ihdr, 4, h);
    ihdr[8] = 8;
    ihdr[9] = 2;
    let empty: Vec<u8> = Vec::new();
    let total = 8 + 25 + 12 + (z.len() as int) + 12;
    let mut out = vec![0u8; total as usize];
    let sig: Vec<u8> = vec![137, 80, 78, 71, 13, 10, 26, 10];
    copy_in(&mut out, 0, &sig);
    let mut at = chunk(&mut out, &t, 8, "IHDR", &ihdr);
    at = chunk(&mut out, &t, at, "IDAT", &z);
    chunk(&mut out, &t, at, "IEND", &empty);
    out
}
