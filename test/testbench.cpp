#include <verilated.h>
#include <verilated_vcd_c.h>
#include "Vtop.h"
#include <iostream>
#include <iomanip>
#include <string>
#include <vector>
#include <cstring>

// Wide vector helpers for Verilator-style wide signals
#define SETBIT(vec, bit)      ((vec)[(bit) / 32] |= (1U << ((bit) % 32)))
#define CLRBIT(vec, bit)      ((vec)[(bit) / 32] &= ~(1U << ((bit) % 32)))
#define TESTBIT(vec, bit)     (((vec)[(bit) / 32] >> ((bit) % 32)) & 1U)

class SHA256Testbench {
private:
    Vtop* dut;
    VerilatedVcdC* tfp;
    vluint64_t sim_time;
    
public:
    SHA256Testbench() : sim_time(0) {
        dut = new Vtop;
        
        // Initialize trace dump
        Verilated::traceEverOn(true);
        tfp = new VerilatedVcdC;
        dut->trace(tfp, 99);
        tfp->open("sha256_trace.vcd");
    }
    
    ~SHA256Testbench() {
        tfp->close();
        delete dut;
        delete tfp;
    }
    
    void reset() {
        // Assert reset
        dut->clock_reset = 0b11; // clock=1, reset=1
        tick();
        
        // Deassert reset
        dut->clock_reset = 0b01; // clock=1, reset=0
        tick();
    }
    
    void tick() {
        // Rising edge
        dut->clock_reset = (dut->clock_reset & 0x2) | 0x1; // clock=1
        dut->eval();
        tfp->dump(sim_time++);
        
        // Falling edge
        dut->clock_reset = (dut->clock_reset & 0x2) | 0x0; // clock=0
        dut->eval();
        tfp->dump(sim_time++);
    }
    
    void set_input_block(const std::vector<uint32_t>& block, bool start = false) {
        if (block.size() != 16) {
            std::cerr << "Block must contain exactly 16 32-bit words" << std::endl;
            return;
        }
        
        // Clear the input first
        VL_ZERO_W(513, dut->i);
        
        // Set the start bit (bit 512, which is the MSB)
        if (start) {
            SETBIT(dut->i, 512);
        }
        
        // Pack the 16 32-bit words into bits 0-511
        // Each word occupies 32 bits
        for (int i = 0; i < 16; i++) {
            uint32_t word = block[i];
            int bit_offset = i * 32;
            
            // Set each bit of the word
            for (int bit = 0; bit < 32; bit++) {
                if (word & (1U << bit)) {
                    SETBIT(dut->i, bit_offset + bit);
                }
            }
        }
    }
    
    std::vector<uint32_t> get_hash_output() {
        std::vector<uint32_t> hash(8);
        
        // Extract the 256-bit hash from the output
        // The output is also a wide signal, so we need to extract bits properly
        for (int i = 0; i < 8; i++) {
            hash[i] = 0;
            // Extract 32 bits starting at bit position i*32
            for (int bit = 0; bit < 32; bit++) {
                if (TESTBIT(dut->o, i * 32 + bit)) {
                    hash[i] |= (1U << bit);
                }
            }
        }
        
        return hash;
    }
    
    bool is_output_valid() {
        // Check if bit 256 is set (valid flag)
        return TESTBIT(dut->o, 256);
    }
    
    void print_hash(const std::vector<uint32_t>& hash) {
        std::cout << "Hash: ";
        for (int i = 0; i < hash.size(); i++) {
            uint32_t word_be = hash[i];
            std::cout << std::hex << std::setw(8) << std::setfill('0') << word_be;
        }
        std::cout << std::dec << std::endl;
    }
};

// Test vector: "abc" message
std::vector<uint32_t> create_test_block_abc() {
    std::vector<uint32_t> block(16, 0);
    
    // "abc" in ASCII with proper SHA-256 padding
    // Message: 'a'=0x61, 'b'=0x62, 'c'=0x63, followed by 0x80 padding bit
    block[0] = 0x61626380; // Big-endian: 'a''b''c' + padding bit
    // All other words are 0 except the length
    block[15] = 0x00000018; // Length = 24 bits (3 bytes) in big-endian
    
    return block;
}

int main(int argc, char** argv) {
    Verilated::commandArgs(argc, argv);
    
    SHA256Testbench tb;
    
    std::cout << "Starting SHA-256 test for 'abc'..." << std::endl;
    
    // Reset the design
    tb.reset();
    
    // Test: "abc"
    std::cout << "\nTesting 'abc' string" << std::endl;
    auto abc_block = create_test_block_abc();
    
    std::cout << "Setting input block..." << std::endl;
    tb.set_input_block(abc_block, true);
    tb.tick(); // Clock the start signal
    
    // Clear start signal
    tb.set_input_block(abc_block, false);
    
    // Wait for computation to complete
    int max_cycles = 200;
    bool found_result = false;
    for (int cycle = 0; cycle < max_cycles; cycle++) {
        tb.tick();
        
        if (tb.is_output_valid()) {
            std::cout << "Output valid at cycle " << cycle << std::endl;
            auto hash = tb.get_hash_output();
            tb.print_hash(hash);
            std::cout << "Expected: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" << std::endl;
            found_result = true;
            break;
        }
    }
    
    if (!found_result) {
        std::cout << "No valid output after " << max_cycles << " cycles" << std::endl;
    }
    
    std::cout << "\nTest completed. Check sha256_trace.vcd for waveform." << std::endl;
    
    return 0;
}
