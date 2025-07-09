use rhdl::prelude::*;
use rhdl_fpga::core::dff::DFF;

// SHA-256 constants (first 32 bits of fractional parts of cube roots of first 64 primes)
const K: [u128; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
];

// Initial hash values (first 32 bits of fractional parts of square roots of first 8 primes)
const H0: [u128; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
];

// SHA-256 state for a single 512-bit block
#[derive(PartialEq, Digital, Default)]
pub struct Sha256State {
    pub h: [Bits<U32>; 8],
    pub a: Bits<U32>,
    pub b: Bits<U32>,
    pub c: Bits<U32>,
    pub d: Bits<U32>,
    pub e: Bits<U32>,
    pub f: Bits<U32>,
    pub g: Bits<U32>,
    pub h_reg: Bits<U32>,
    pub round: Bits<U128>, // 0-63 rounds
    pub done: bool,
}

#[derive(PartialEq, Digital)]
pub struct Sha256Input {
    pub block: [Bits<U32>; 16], // 512-bit input block (16 x 32-bit words)
    pub start: bool,
}

#[derive(PartialEq, Digital)]
pub struct Sha256Output {
    pub done: bool,           // Computation complete
    //pub hash_lsb: Bits<U32>,   // Just the lowest 8 bits of first hash word
}

#[derive(Clone, Synchronous, SynchronousDQ)]
pub struct Sha256Core {
    state: DFF<Sha256State>,
    // Store message schedule in separate DFF to avoid large arrays in main state
    w: [DFF<Bits<U32>>; 16], // Only store current 16 words, compute others on-the-fly
}

impl Default for Sha256Core {
    fn default() -> Self {
        Self {
            state: DFF::new(Sha256State::default()),
            w: [
                DFF::new(bits(0)), DFF::new(bits(0)), DFF::new(bits(0)), DFF::new(bits(0)),
                DFF::new(bits(0)), DFF::new(bits(0)), DFF::new(bits(0)), DFF::new(bits(0)),
                DFF::new(bits(0)), DFF::new(bits(0)), DFF::new(bits(0)), DFF::new(bits(0)),
                DFF::new(bits(0)), DFF::new(bits(0)), DFF::new(bits(0)), DFF::new(bits(0)),
            ],
        }
    }
}

impl SynchronousIO for Sha256Core {
    type I = Sha256Input;
    //type O = Sha256Output;
    type O = Sha256Output;
    type Kernel = kernel;
}

// Helper function for right rotation
#[kernel]
fn rotr(x: Bits<U32>, n: u128) -> Bits<U32> {
    let n = n & 31; // Ensure n is in range 0-31
    (x >> n) | (x << (32 - n))
}

#[kernel]
fn ch(x: Bits<U32>, y: Bits<U32>, z: Bits<U32>) -> Bits<U32> {
    (x & y) ^ (!x & z)
}

#[kernel]
fn maj(x: Bits<U32>, y: Bits<U32>, z: Bits<U32>) -> Bits<U32> {
    (x & y) ^ (x & z) ^ (y & z)
}

#[kernel]
fn sigma0(x: Bits<U32>) -> Bits<U32> {
    rotr(x, 2) ^ rotr(x, 13) ^ rotr(x, 22)
}

#[kernel]
fn sigma1(x: Bits<U32>) -> Bits<U32> {
    rotr(x, 6) ^ rotr(x, 11) ^ rotr(x, 25)
}

#[kernel]
fn gamma0(x: Bits<U32>) -> Bits<U32> {
    rotr(x, 7) ^ rotr(x, 18) ^ (x >> 3)
}

#[kernel]
fn gamma1(x: Bits<U32>) -> Bits<U32> {
    rotr(x, 17) ^ rotr(x, 19) ^ (x >> 10)
}

// Get K constant for current round
#[kernel]
fn get_k(round: Bits<U128>) -> Bits<U32> {
    let r = round.raw() as usize;
    if r < 64 {
        bits(K[r])
    } else {
        bits(0)
    }
}

// Get W value for current round - compute on the fly for rounds 16+
#[kernel]
fn get_w(round: Bits<U128>, w0: Bits<U32>, w1: Bits<U32>, w9: Bits<U32>, w14: Bits<U32>) -> Bits<U32> {
    let r = round.raw();
    if r < 16 {
        w0 // For rounds 0-15, use stored value
    } else {
        // For rounds 16+, compute: W[i] = gamma1(W[i-2]) + W[i-7] + gamma0(W[i-15]) + W[i-16]
        gamma1(w14) + w9 + gamma0(w1) + w0
    }
}

#[kernel]
pub fn kernel(_cr: ClockReset, input: Sha256Input, q: Q) -> (Sha256Output, D) {
    let mut d = D::dont_care();

    if input.start {
        // Initialize new hash computation
        d.state = Sha256State {
            h: [
                bits(H0[0]), bits(H0[1]), bits(H0[2]), bits(H0[3]),
                bits(H0[4]), bits(H0[5]), bits(H0[6]), bits(H0[7])
            ],
            round: bits(0),
            done: false,
            a: bits(H0[0]),
            b: bits(H0[1]),
            c: bits(H0[2]),
            d: bits(H0[3]),
            e: bits(H0[4]),
            f: bits(H0[5]),
            g: bits(H0[6]),
            h_reg: bits(H0[7]),
        };
        
        // Initialize message schedule with input block
        d.w[0] = input.block[0];
        d.w[1] = input.block[1];
        d.w[2] = input.block[2];
        d.w[3] = input.block[3];
        d.w[4] = input.block[4];
        d.w[5] = input.block[5];
        d.w[6] = input.block[6];
        d.w[7] = input.block[7];
        d.w[8] = input.block[8];
        d.w[9] = input.block[9];
        d.w[10] = input.block[10];
        d.w[11] = input.block[11];
        d.w[12] = input.block[12];
        d.w[13] = input.block[13];
        d.w[14] = input.block[14];
        d.w[15] = input.block[15];
        
    } else if !q.state.done {
        let round_val = q.state.round.raw();
        
        // Copy current state
        d.state = q.state;
        
        // Shift message schedule window for rounds 16+
        if round_val >= 16 {
            d.w[0] = q.w[1];
            d.w[1] = q.w[2];
            d.w[2] = q.w[3];
            d.w[3] = q.w[4];
            d.w[4] = q.w[5];
            d.w[5] = q.w[6];
            d.w[6] = q.w[7];
            d.w[7] = q.w[8];
            d.w[8] = q.w[9];
            d.w[9] = q.w[10];
            d.w[10] = q.w[11];
            d.w[11] = q.w[12];
            d.w[12] = q.w[13];
            d.w[13] = q.w[14];
            d.w[14] = q.w[15];
            d.w[15] = get_w(q.state.round, q.w[0], q.w[1], q.w[9], q.w[14]);
        } else {
            // Keep current message schedule
            d.w[0] = q.w[0];
            d.w[1] = q.w[1];
            d.w[2] = q.w[2];
            d.w[3] = q.w[3];
            d.w[4] = q.w[4];
            d.w[5] = q.w[5];
            d.w[6] = q.w[6];
            d.w[7] = q.w[7];
            d.w[8] = q.w[8];
            d.w[9] = q.w[9];
            d.w[10] = q.w[10];
            d.w[11] = q.w[11];
            d.w[12] = q.w[12];
            d.w[13] = q.w[13];
            d.w[14] = q.w[14];
            d.w[15] = q.w[15];
        }
        
        // Main compression function for rounds 0-63
        if round_val < 64 {
            let current_w = if round_val < 16 {
                let idx = round_val as usize;
                if idx == 0 { q.w[0] }
                else if idx == 1 { q.w[1] }
                else if idx == 2 { q.w[2] }
                else if idx == 3 { q.w[3] }
                else if idx == 4 { q.w[4] }
                else if idx == 5 { q.w[5] }
                else if idx == 6 { q.w[6] }
                else if idx == 7 { q.w[7] }
                else if idx == 8 { q.w[8] }
                else if idx == 9 { q.w[9] }
                else if idx == 10 { q.w[10] }
                else if idx == 11 { q.w[11] }
                else if idx == 12 { q.w[12] }
                else if idx == 13 { q.w[13] }
                else if idx == 14 { q.w[14] }
                else { q.w[15] }
            } else {
                d.w[15] // Use the computed value
            };
            
            let s1 = sigma1(q.state.e);
            let ch_result = ch(q.state.e, q.state.f, q.state.g);
            let temp1 = q.state.h_reg + s1 + ch_result + get_k(q.state.round) + current_w;
            
            let s0 = sigma0(q.state.a);
            let maj_result = maj(q.state.a, q.state.b, q.state.c);
            let temp2 = s0 + maj_result;
            
            d.state.h_reg = q.state.g;
            d.state.g = q.state.f;
            d.state.f = q.state.e;
            d.state.e = q.state.d + temp1;
            d.state.d = q.state.c;
            d.state.c = q.state.b;
            d.state.b = q.state.a;
            d.state.a = temp1 + temp2;
            
            d.state.round = bits(round_val + 1);
            
            // Check if we've completed all 64 rounds
            if round_val == 63 {
                // Add compressed chunk to current hash value
                d.state.h[0] = q.state.h[0] + d.state.a;
                d.state.h[1] = q.state.h[1] + d.state.b;
                d.state.h[2] = q.state.h[2] + d.state.c;
                d.state.h[3] = q.state.h[3] + d.state.d;
                d.state.h[4] = q.state.h[4] + d.state.e;
                d.state.h[5] = q.state.h[5] + d.state.f;
                d.state.h[6] = q.state.h[6] + d.state.g;
                d.state.h[7] = q.state.h[7] + d.state.h_reg;
                
                d.state.done = true;
            }
        }
    } else {
        // Keep current state and keep outputting the final hash
        d.state = q.state;
        d.w[0] = q.w[0];
        d.w[1] = q.w[1];
        d.w[2] = q.w[2];
        d.w[3] = q.w[3];
        d.w[4] = q.w[4];
        d.w[5] = q.w[5];
        d.w[6] = q.w[6];
        d.w[7] = q.w[7];
        d.w[8] = q.w[8];
        d.w[9] = q.w[9];
        d.w[10] = q.w[10];
        d.w[11] = q.w[11];
        d.w[12] = q.w[12];
        d.w[13] = q.w[13];
        d.w[14] = q.w[14];
        d.w[15] = q.w[15];
    }

    //let hash_out = q.state.h;
    let output = Sha256Output {
        done: q.state.done,
        //hash_lsb: hash_out[0],
    };
    
    (output, d)
}

fn pad_message_to_block(message: &[u8]) -> [Bits<U32>; 16] {
    let mut block = [bits(0u128); 16];
    let message_len = message.len();
    let message_len_bits = (message_len * 8) as u64;
    
    // Copy message bytes into the block (4 bytes per u32, big-endian)
    let mut byte_pos = 0;
    for i in 0..16 {
        let mut word = 0u32;
        for j in 0..4 {
            if byte_pos < message_len {
                word |= (message[byte_pos] as u32) << (24 - j * 8);
                byte_pos += 1;
            } else if byte_pos == message_len {
                // Add the padding bit (0x80)
                word |= 0x80 << (24 - j * 8);
                byte_pos += 1;
            }
            // Otherwise leave as 0 (implicit padding)
        }
        block[i] = bits(word as u128);
    }
    
    // Add length in bits to the last 64 bits (last 2 u32s)
    // SHA-256 uses big-endian format
    block[14] = bits((message_len_bits >> 32) as u128); // High 32 bits
    block[15] = bits(message_len_bits as u128);         // Low 32 bits
    
    block
}

fn main() -> Result<(), RHDLError> {
    let test_block = pad_message_to_block("abc".as_bytes());
    
    // Create input sequence: start pulse, then wait for completion
    let inputs = std::iter::once(Sha256Input {
        block: test_block,
        start: true,
    })
    .chain(std::iter::repeat(Sha256Input {
        block: test_block,
        start: false,
    }))
    .take(70) // Need ~65 cycles for all rounds plus some extra
    .with_reset(1)
    .clock_pos_edge(100);
    
    let uut = Sha256Core::default();
    let _vcd = uut.run(inputs)?.collect::<Vcd>();

    Ok(())
}